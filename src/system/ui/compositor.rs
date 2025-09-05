use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::window::{Window, WindowHandle};
use crate::system::ui::canvas::Canvas;
use crate::libs::gfx::two_d::{Rect, Rgb565};
use crate::system::kernel::config::resources::{
    FRAME_BUFFER_HEIGHT, FRAME_BUFFER_SIZE, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR
};
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::resources::input_channels::INPUT_CHANNEL_POOL;

use alloc::boxed::Box;
use defmt::{debug, error, info, warn, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::geometry::{Point as EgPoint, Size as EgSize};
use heapless::Vec;
use libm::sqrtf;
use micromath::F32Ext;

/// Maximum supported windows in the compositor
const MAX_WINDOWS: usize = 8;
/// Maximum pending redraw requests
const MAX_REDRAW_REQUESTS: usize = 4;

/// Display view configuration
#[derive(Clone, Copy, Debug, PartialEq, Format)]
pub enum ViewMode {
    /// Single window fills entire display
    Single,
    /// Two windows side-by-side
    Split,
}

/// Animation transition directions
#[derive(Clone, Copy, Debug, Format)]
pub enum TransitionDirection {
    Previous,
    Next,
}

/// Animation interpolation function signature
type EasingFn = fn(f32) -> f32;

/// Animation configuration for smooth transitions
#[derive(Clone, Copy)]
pub struct AnimationConfig {
    /// Total animation steps for smooth motion
    pub steps: usize,
    /// Delay between animation frames in milliseconds
    pub frame_delay_ms: u64,
    /// Easing function for smooth interpolation
    pub easing_fn: EasingFn,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            steps: 8,
            frame_delay_ms: 25,
            easing_fn: ease_in_out_cubic,
        }
    }
}

/// Animation parameters for a single frame
#[derive(Clone, Copy, Debug)]
pub struct AnimationFrame {
    /// Horizontal offset for source window
    pub source_x: i32,
    /// Vertical offset for source window
    pub source_y: i32,
    /// Horizontal offset for target window
    pub target_x: i32,
    /// Vertical offset for target window
    pub target_y: i32,
    /// Scale factor for zoom effect (1.0 = normal size)
    pub scale: f32,
}

/// Trait for different animation types
pub trait WindowAnimation {
    /// Generate animation frame for given progress (0.0 to 1.0)
    fn animate_frame(&self, progress: f32, direction: TransitionDirection) -> AnimationFrame;

    /// Animation display name for debugging
    fn name(&self) -> &'static str;
}

/// Slide animation with zoom effect
#[derive(Default)]
pub struct SlideZoomAnimation;

impl WindowAnimation for SlideZoomAnimation {
    fn animate_frame(&self, progress: f32, direction: TransitionDirection) -> AnimationFrame {
        let width = FRAME_BUFFER_WIDTH as i32;
        let offset = (progress * width as f32) as i32;

        // Add subtle zoom out at mid-transition for depth effect
        let zoom_factor = if progress < 0.5 {
            1.0 - (progress * 0.1) // Zoom out slightly
        } else {
            0.95 + ((progress - 0.5) * 0.1) // Zoom back in
        };

        match direction {
            TransitionDirection::Previous => AnimationFrame {
                source_x: -offset,
                source_y: 0,
                target_x: -offset + width,
                target_y: 0,
                scale: zoom_factor,
            },
            TransitionDirection::Next => AnimationFrame {
                source_x: offset,
                source_y: 0,
                target_x: offset - width,
                target_y: 0,
                scale: zoom_factor,
            },
        }
    }

    fn name(&self) -> &'static str {
        "SlideZoom"
    }
}

/// Fade transition animation
#[derive(Default)]
pub struct FadeAnimation;

impl WindowAnimation for FadeAnimation {
    fn animate_frame(&self, _progress: f32, _direction: TransitionDirection) -> AnimationFrame {
        // For fade, both windows occupy same position, alpha handled elsewhere
        AnimationFrame {
            source_x: 0,
            source_y: 0,
            target_x: 0,
            target_y: 0,
            scale: 1.0,
        }
    }

    fn name(&self) -> &'static str {
        "Fade"
    }
}

// Note: legacy PixelColor-based blitting removed. We operate directly on Rgb565 buffers.

/// High-performance canvas blitting operations
pub struct BlitOperations;

impl BlitOperations {
    /// Copy entire source canvas to destination with offset
    pub fn copy_full<'a>(
        dest: &mut Canvas<'a>,
        source: &Canvas<'a>,
        offset_x: i32,
        offset_y: i32,
    ) {
        let timer = Instant::now();
        let (dest_w, dest_h) = (dest.width(), dest.height());
        let (src_w, src_h) = (source.width(), source.height());

        Self::copy_pixels_bounded(
            dest, source,
            0, 0, src_w, src_h,
            offset_x, offset_y,
            dest_w, dest_h
        );

        debug!("Full blit: {} μs", timer.elapsed().as_micros());
    }

    /// Copy specific region of source canvas to destination
    pub fn copy_region<'a>(
        dest: &mut Canvas<'a>,
        source: &Canvas<'a>,
        region: Rect,
        offset_x: i32,
        offset_y: i32,
    ) {
        let timer = Instant::now();
        let (dest_w, dest_h) = (dest.width(), dest.height());
        let (src_w, src_h) = (source.width(), source.height());

        // Calculate bounded region coordinates
        let region_x = region.top_left.x.max(0) as u32;
        let region_y = region.top_left.y.max(0) as u32;
        let region_w = (region.top_left.x as u32 + region.size.width).min(src_w);
        let region_h = (region.top_left.y as u32 + region.size.height).min(src_h);

        if region_x >= region_w || region_y >= region_h {
            debug!("Region blit skipped: empty bounds");
            return;
        }

        Self::copy_pixels_bounded(
            dest, source,
            region_x, region_y, region_w, region_h,
            offset_x, offset_y,
            dest_w, dest_h
        );

        debug!("Region blit: {} μs", timer.elapsed().as_micros());
    }

    /// Internal bounds-checked pixel copying
    fn copy_pixels_bounded<'a>(
        dest: &mut Canvas<'a>,
        source: &Canvas<'a>,
        src_x: u32, src_y: u32, src_w: u32, src_h: u32,
        offset_x: i32, offset_y: i32,
        dest_w: u32, dest_h: u32,
    ) {
        for y in src_y..src_h {
            let dest_y = y as i32 + offset_y;
            if dest_y < 0 || dest_y >= dest_h as i32 {
                continue;
            }

            for x in src_x..src_w {
                let dest_x = x as i32 + offset_x;
                if dest_x < 0 || dest_x >= dest_w as i32 {
                    continue;
                }

                let src_pixel_idx = (y * source.width() + x) as usize;
                let dest_pixel_idx = (dest_y as u32 * dest_w + dest_x as u32) as usize;
                let src_byte_idx = src_pixel_idx * 2;
                let dest_byte_idx = dest_pixel_idx * 2;
                if src_byte_idx + 1 < source.buffer().len() && dest_byte_idx + 1 < dest.buffer_mut().len() {
                    dest.buffer_mut()[dest_byte_idx] = source.buffer()[src_byte_idx];
                    dest.buffer_mut()[dest_byte_idx + 1] = source.buffer()[src_byte_idx + 1];
                }
            }
        }
    }
}

/// Space-grade UI compositor for window management and rendering
pub struct UICompositor {
    /// Active window stack
    windows: Vec<Window, MAX_WINDOWS>,
    /// Index of currently focused window
    focused_window_idx: usize,
    /// Current display mode (single/split view)
    display_mode: ViewMode,
    /// Display hardware interface
    display_driver: Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
    /// Pending redraw requests for dirty windows
    pending_redraws: Vec<WindowHandle, MAX_REDRAW_REQUESTS>,
    /// Animation configuration
    animation_config: AnimationConfig,
}

impl UICompositor {
    /// Initialize new compositor instance
    pub fn new() -> Self {
        info!("Initializing UI Compositor");
        Self {
            windows: Vec::new(),
            focused_window_idx: 0,
            display_mode: ViewMode::Single,
            display_driver: None,
            pending_redraws: Vec::new(),
            animation_config: AnimationConfig::default(),
        }
    }

    /// Attach hardware display driver
    pub fn attach_display(
        &mut self,
        display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) {
        self.display_driver = Some(display);
        info!("Display driver attached");
    }

    /// Configure animation parameters
    pub fn set_animation_config(&mut self, config: AnimationConfig) {
        self.animation_config = config;
        debug!("Animation config updated: steps={}, delay={}ms",
               config.steps, config.frame_delay_ms);
    }

    /// Queue window for redraw
    pub fn request_redraw(&mut self, window_handle: WindowHandle) {
        if !self.pending_redraws.contains(&window_handle) {
            if self.pending_redraws.push(window_handle).is_err() {
                warn!("Redraw queue full, dropping request for {:?}", window_handle);
            } else {
                debug!("Redraw queued for window {:?}", window_handle);
            }
        }
    }

    /// Process all pending redraws and update display
    pub async fn process_redraws(&mut self) {
        if self.pending_redraws.is_empty() {
            return;
        }

        let render_start = Instant::now();

        if let Some(display) = self.display_driver {
            // Allocate working buffer for composition
            let mut frame_buffer = [0u8; FRAME_BUFFER_SIZE];
            let mut composite_canvas = Canvas::new(
                FRAME_BUFFER_WIDTH,
                FRAME_BUFFER_HEIGHT
            );
            composite_canvas.set_resources(&mut frame_buffer);

            let current_mode = self.display_mode;
            let dirty_regions = self.get_focused_window_dirty_regions();

            // Compose final frame
            self.compose_frame(&mut composite_canvas).await;

            // Update display efficiently
            let mut display_lock = display.lock().await;
            if current_mode == ViewMode::Single && !dirty_regions.is_empty() {
                // Partial updates for better performance: draw each dirty region
                for region in dirty_regions.iter() {
                    display_lock.draw_region(
                        composite_canvas.buffer(),
                        Rectangle::new(
                            EgPoint::new(region.top_left.x, region.top_left.y),
                            EgSize::new(region.size.width, region.size.height),
                        ),
                        FRAME_SCALE_FACTOR,
                    ).await;
                }
                self.clear_focused_window_dirty_regions();
            } else {
                // Full frame update
                display_lock.draw(
                    composite_canvas.buffer(),
                    FRAME_SCALE_FACTOR
                ).await;
            }
        }

        self.pending_redraws.clear();
        debug!("Frame rendered in {} μs", render_start.elapsed().as_micros());
    }

    /// Compose windows into final frame buffer
    async fn compose_frame<'a>(&mut self, output_canvas: &mut Canvas<'a>) {
        // Clear to black background
        output_canvas.clear_rgb(Rgb565::BLACK);

        if self.windows.is_empty() {
            return;
        }

        let focused_window = &mut self.windows[self.focused_window_idx];

        if let Some(window_canvas) = focused_window.canvas().as_mut() {
            match self.display_mode {
                ViewMode::Single => {
                    // Render single focused window
                    let regions = window_canvas.dirty_regions();
                    if !regions.is_empty() {
                        for r in regions {
                            BlitOperations::copy_region(output_canvas, window_canvas, *r, 0, 0);
                        }
                    } else {
                        BlitOperations::copy_full(output_canvas, window_canvas, 0, 0);
                    }
                }
                ViewMode::Split => {
                    // Render split view with two windows
                    BlitOperations::copy_full(output_canvas, window_canvas, 0, 0);

                    let split_window_idx = self.calculate_split_window_index();
                    if let Some(split_canvas) = self.windows[split_window_idx].canvas().as_mut() {
                        let split_x_offset = FRAME_BUFFER_WIDTH as i32 / 2;
                        BlitOperations::copy_full(output_canvas, split_canvas, split_x_offset, 0);
                    }
                }
            }
        }
    }

    /// Get handle of currently focused window
    pub fn focused_window_handle(&self) -> Option<WindowHandle> {
        if self.windows.is_empty() {
            None
        } else {
            Some(self.windows[self.focused_window_idx].handle())
        }
    }

    /// Get mutable reference to window by handle
    pub fn get_window_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }

    /// Create new window with specified dimensions
    pub async fn create_window(
        &mut self,
        width: u32,
        height: u32,
        window_id: usize,
    ) -> Option<WindowHandle> {
        let window = Window::new(width, height, window_id).await;
        let handle = window.handle();

        match self.windows.push(window) {
            Ok(_) => {
                info!("Window created: id={}, size={}x{}", window_id, width, height);

                // Allocate resources if this is the first window
                if self.windows.len() == 1 {
                    self.allocate_active_window_resources().await;
                }

                Some(handle)
            }
            Err(_) => {
                error!("Failed to create window: compositor full");
                None
            }
        }
    }

    /// Toggle between single and split view modes
    pub async fn toggle_display_mode(&mut self) {
        self.display_mode = match self.display_mode {
            ViewMode::Single => ViewMode::Split,
            ViewMode::Split => ViewMode::Single,
        };

        info!("Display mode: {:?}", self.display_mode);
        self.allocate_active_window_resources().await;
    }

    /// Switch to next window with smooth animation
    pub async fn animate_to_next_window(&mut self) {
        self.animate_window_transition(TransitionDirection::Next).await;
    }

    /// Switch to previous window with smooth animation
    pub async fn animate_to_previous_window(&mut self) {
        self.animate_window_transition(TransitionDirection::Previous).await;
    }

    /// Execute smooth animated transition between windows
    async fn animate_window_transition(&mut self, direction: TransitionDirection) {
        if self.windows.len() < 2 || self.display_mode == ViewMode::Split {
            debug!("Animation skipped: insufficient windows or split mode");
            return;
        }

        let animation = SlideZoomAnimation::default();
        let source_idx = self.focused_window_idx;
        let target_idx = self.calculate_transition_target_index(direction);

        info!("Animating {} from window {} to {}",
              animation.name(), source_idx, target_idx);

        // Ensure target window has allocated resources
        self.prepare_window_for_transition(target_idx).await;

        // Update focus before animation
        self.update_focused_window_index(direction);

        // Execute smooth animation
        self.execute_animation(&animation, direction, source_idx, target_idx).await;

        // Clean up resources after transition
        self.allocate_active_window_resources().await;
    }

    /// Execute the actual animation frames
    async fn execute_animation(
        &mut self,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
        source_idx: usize,
        target_idx: usize,
    ) {
        let mut composition_buffer = [0u8; FRAME_BUFFER_SIZE];
        let mut canvas = Canvas::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);
        canvas.set_resources(&mut composition_buffer);

        // Temporarily take ownership of canvases
        let source_canvas = self.windows[source_idx].canvas().take()
            .expect("Source canvas unavailable");
        let target_canvas = self.windows[target_idx].canvas().take()
            .expect("Target canvas unavailable");

        // Animate through all steps
        for step in 0..=self.animation_config.steps {
            let frame_timer = Instant::now();
            let progress = step as f32 / self.animation_config.steps as f32;
            let eased_progress = (self.animation_config.easing_fn)(progress);

            let frame = animation.animate_frame(eased_progress, direction);

            // Clear and compose frame
            canvas.clear_rgb(Rgb565::BLACK);
            BlitOperations::copy_full(&mut canvas, &source_canvas, frame.source_x, frame.source_y);
            BlitOperations::copy_full(&mut canvas, &target_canvas, frame.target_x, frame.target_y);

            // Display frame
            if let Some(display) = self.display_driver {
                let mut display_lock = display.lock().await;
                display_lock.draw(canvas.buffer(), FRAME_SCALE_FACTOR).await;
            }

            debug!("Animation step {}: {} μs", step, frame_timer.elapsed().as_micros());
            Timer::after(Duration::from_millis(self.animation_config.frame_delay_ms)).await;
        }

        // Restore canvases
        self.windows[source_idx].canvas().replace(source_canvas);
        self.windows[target_idx].canvas().replace(target_canvas);
    }

    /// Allocate resources for currently active windows
    async fn allocate_active_window_resources(&mut self) {
        if self.windows.is_empty() {
            return;
        }

        let active_indices = self.calculate_active_window_indices();

        // Release resources from inactive windows
        for (idx, window) in self.windows.iter_mut().enumerate() {
            if !active_indices.contains(&idx) && window.framebuffer_id().is_some() {
                debug!("Releasing resources for inactive window {}", idx);
                window.relax();
            }
        }

        // Allocate resources for active windows
        for &idx in &active_indices {
            self.allocate_window_resources(idx).await;
        }
    }

    /// Allocate resources for specific window
    async fn allocate_window_resources(&mut self, window_idx: usize) {
        let window = &mut self.windows[window_idx];

        if window.framebuffer_id().is_some() {
            return; // Already has resources
        }

        debug!("Allocating resources for window {}", window_idx);

        match FRAMEBUFFER_POOL.allocate().await {
            Some(framebuffer) => {
                match INPUT_CHANNEL_POOL.allocate().await {
                    Some(input_channel) => {
                        window.set_resources(framebuffer, input_channel).await;
                    }
                    None => {
                        warn!("Input channel allocation failed for window {}", window_idx);
                        FRAMEBUFFER_POOL.release(&framebuffer);
                    }
                }
            }
            None => {
                warn!("Framebuffer allocation failed for window {}", window_idx);
            }
        }
    }

    /// Calculate which windows need resources allocated
    fn calculate_active_window_indices(&self) -> Vec<usize, 4> {
        let mut active = Vec::new();
        let window_count = self.windows.len();

        if window_count == 0 {
            return active;
        }

        // Always include focused window
        active.push(self.focused_window_idx).ok();

        // Add adjacent windows for smooth transitions
        if window_count > 1 {
            let prev_idx = (self.focused_window_idx + window_count - 1) % window_count;
            let next_idx = (self.focused_window_idx + 1) % window_count;

            active.push(prev_idx).ok();
            active.push(next_idx).ok();
        }

        // Add split window if in split mode
        if self.display_mode == ViewMode::Split && window_count > 1 {
            let split_idx = self.calculate_split_window_index();
            active.push(split_idx).ok();
        }

        active
    }

    // Helper methods
    fn calculate_split_window_index(&self) -> usize {
        (self.focused_window_idx + 1) % self.windows.len()
    }

    fn calculate_transition_target_index(&self, direction: TransitionDirection) -> usize {
        let window_count = self.windows.len();
        match direction {
            TransitionDirection::Previous => {
                (self.focused_window_idx + window_count - 1) % window_count
            }
            TransitionDirection::Next => {
                (self.focused_window_idx + 1) % window_count
            }
        }
    }

    fn update_focused_window_index(&mut self, direction: TransitionDirection) {
        self.focused_window_idx = self.calculate_transition_target_index(direction);
    }

    async fn prepare_window_for_transition(&mut self, target_idx: usize) {
        self.allocate_window_resources(target_idx).await;
    }

    fn get_focused_window_dirty_regions(&mut self) -> heapless::Vec<Rect, 8> {
        let mut out: heapless::Vec<Rect, 8> = heapless::Vec::new();
        if let Some(canvas) = self.windows[self.focused_window_idx].canvas().as_mut() {
            for r in canvas.dirty_regions() {
                out.push(*r).ok();
            }
        }
        out
    }

    fn clear_focused_window_dirty_regions(&mut self) {
        if let Some(canvas) = self.windows[self.focused_window_idx].canvas().as_mut() {
            canvas.flush();
        }
    }

    /// Poll for input events from specified window
    pub fn poll_window_input(
        &mut self,
        handle: WindowHandle,
    ) -> Option<crate::system::services::human_input_srv::HumanInputEvent> {
        self.get_window_mut(handle)
            .and_then(|window| window.input_receiver())
            .and_then(|receiver| receiver.try_receive().ok())
    }

    /// Check if window is currently focused
    pub fn is_window_focused(&self, handle: WindowHandle) -> bool {
        !self.windows.is_empty() &&
            self.windows[self.focused_window_idx].handle() == handle
    }
}

/// Smooth cubic easing function for natural motion
pub(crate) fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let shifted = 2.0 * t - 2.0;
        1.0 + shifted * shifted * shifted / 2.0
    }
}

/// Circular easing for dramatic effect
pub(crate) fn ease_in_out_circular(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - sqrtf(1.0 - 4.0 * t * t))
    } else {
        0.5 * (sqrtf(1.0 - (2.0 * t - 2.0).powf(2.0)) + 1.0)
    }
}

/// Bouncy easing for playful transitions
pub(crate) fn ease_out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let shifted = t - 1.5 / D1;
        N1 * shifted * shifted + 0.75
    } else if t < 2.5 / D1 {
        let shifted = t - 2.25 / D1;
        N1 * shifted * shifted + 0.9375
    } else {
        let shifted = t - 2.625 / D1;
        N1 * shifted * shifted + 0.984375
    }
}