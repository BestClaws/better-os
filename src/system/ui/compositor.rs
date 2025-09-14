use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::window::WindowHandle;
use crate::system::ui::canvas::DrawingSurface as Canvas;
use crate::libs::gfx::two_d::{Rect, Rgb565};
use crate::system::kernel::config::resources::{
    FRAME_BUFFER_HEIGHT, FRAME_BUFFER_SIZE, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR
};
use crate::system::ui::window_manager::WindowManager;
use crate::system::services::display_service::DisplayService;
use crate::system::input::dispatcher::{SUI_ACK_CH, SUI_EVENT_CH};
use crate::system::input::types::{HighLevelEvent, MotionEvent, TouchAction};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
};

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec as AllocVec;
use defmt::{debug, info, warn, Format};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
use libm::sqrtf;
// micromath::F32Ext not required; easing uses libm and core ops only

/// Maximum order list maintained by compositor (matches WindowManager capacity)
const MAX_WINDOWS: usize = 8;
/// Maximum pending redraw requests
const MAX_REDRAW_REQUESTS: usize = 4;

/// Animation transition directions
#[derive(Clone, Copy, Debug, Format)]
pub enum TransitionDirection {
    Previous,
    Next,
}

/// Animation interpolation function signature
type EasingFn = fn(f32) -> f32;

/// Update strategy for display rendering optimization
#[derive(Debug)]
enum UpdateStrategy {
    /// Full screen update (more efficient for large changes)
    FullScreen,
    /// Partial update with specific regions
    Partial(heapless::Vec<Rect, 8>),
}

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

/// High-performance canvas blitting operations optimized for embedded systems.
/// 
/// This blitter provides space-grade performance with:
/// - Row-based clipped copies with minimal overhead
/// - Pre-computed bounds checking to avoid per-pixel validation
/// - Optimized memory copy operations using memcpy where possible
/// - Efficient dirty region management for partial updates
pub struct CanvasBlitter;

impl CanvasBlitter {
    /// Copy entire source canvas to destination with offset.
    /// Internally performs row-based memcpy with precomputed clipping.
    pub fn copy_full<'d, 's>(
        dest: &mut Canvas<'d>,
        source: &Canvas<'s>,
        offset_x: i32,
        offset_y: i32,
        bytes_per_pixel: usize,
    ) {
        let timer = Instant::now();
        let (dest_w, dest_h) = (dest.width(), dest.height());
        let (src_w, src_h) = (source.width(), source.height());

        Self::copy_rows_clipped(
            dest, source,
            0, 0, src_w, src_h,
            offset_x, offset_y,
            dest_w, dest_h,
            bytes_per_pixel,
        );

        debug!("Full blit: {} μs", timer.elapsed().as_micros());
    }

    /// Copy specific region of source canvas to destination
    pub fn copy_region<'d, 's>(
        dest: &mut Canvas<'d>,
        source: &Canvas<'s>,
        region: Rect,
        offset_x: i32,
        offset_y: i32,
        bytes_per_pixel: usize,
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

        Self::copy_rows_clipped(
            dest, source,
            region_x, region_y, region_w, region_h,
            offset_x, offset_y,
            dest_w, dest_h,
            bytes_per_pixel,
        );

        debug!("Region blit: {} μs", timer.elapsed().as_micros());
    }

    /// Internal: fast row-based copy with clipping computed once per row.
    fn copy_rows_clipped<'d, 's>(
        dest: &mut Canvas<'d>,
        source: &Canvas<'s>,
        src_x0: u32, src_y0: u32, src_x1: u32, src_y1: u32,
        offset_x: i32, offset_y: i32,
        dest_w: u32, dest_h: u32,
        bytes_per_pixel: usize,
    ) {
        // Convert to i32 for math once
        let dest_w_i = dest_w as i32;
        let dest_h_i = dest_h as i32;
        let src_w = source.width();

        // Walk rows once; each row does at most two bound checks and one memcpy
        for sy in src_y0..src_y1 {
            let dy = sy as i32 + offset_y;
            if dy < 0 || dy >= dest_h_i { continue; }

            // Compute horizontal clip for this row
            let dx0 = src_x0 as i32 + offset_x;
            let dx1 = src_x1 as i32 + offset_x;
            let clip_x0 = dx0.max(0).min(dest_w_i);
            let clip_x1 = dx1.max(0).min(dest_w_i);
            if clip_x0 >= clip_x1 { continue; }

            // Map back to source x range based on clipping
            let sx0 = (clip_x0 - offset_x).max(src_x0 as i32) as u32;
            let sx1 = (clip_x1 - offset_x).min(src_x1 as i32) as u32;
            if sx0 >= sx1 { continue; }

            let pixels = (sx1 - sx0) as usize;
            let bytes = pixels * bytes_per_pixel;

            // Compute byte indices once and copy the entire run
            let src_first_pixel = (sx0 + sy * src_w) as usize;
            let dst_first_pixel = (clip_x0 as u32 + dy as u32 * dest_w) as usize;
            let src_byte = src_first_pixel * bytes_per_pixel;
            let dst_byte = dst_first_pixel * bytes_per_pixel;

            // Safety: all indices computed with clipping; copy in one slice move
            let src_slice = &source.buffer()[src_byte .. src_byte + bytes];
            let dst_slice = &mut dest.buffer_mut()[dst_byte .. dst_byte + bytes];
            dst_slice.copy_from_slice(src_slice);
        }
    }
}

/// Space-grade UI compositor focused on composition and transitions
pub struct UICompositor {
    /// Logical order of windows (by handle)
    windows_order: heapless::Vec<WindowHandle, MAX_WINDOWS>,
    /// Index of currently focused window within `windows_order`
    current_index: usize,
    /// Display service (preferred)
    display_service: Option<&'static DisplayService>,
    /// Pending redraw requests for dirty windows
    pending_redraws: heapless::Vec<WindowHandle, MAX_REDRAW_REQUESTS>,
    /// Animation configuration
    animation_config: AnimationConfig,
}

impl UICompositor {
    /// Initialize new compositor instance
    pub fn new() -> Self {
        debug!("Initializing UI Compositor");
        Self {
            windows_order: heapless::Vec::new(),
            current_index: 0,
            display_service: None,
            pending_redraws: heapless::Vec::new(),
            animation_config: AnimationConfig::default(),
        }
    }

    /// Attach DisplayService abstraction
    pub fn attach_display_service(&mut self, service: &'static DisplayService) {
        self.display_service = Some(service);
        debug!("Display service attached");
    }

    /// Configure animation parameters
    pub fn set_animation_config(&mut self, config: AnimationConfig) {
        self.animation_config = config;
        debug!("Animation config updated: steps={}, delay={}ms",
               config.steps, config.frame_delay_ms);
    }

    /// Register a window handle into the compositor's logical order.
    /// The first registered window becomes focused by default.
    pub async fn register_window(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        if self.windows_order.len() >= MAX_WINDOWS {
            warn!("Compositor window order full; cannot register more");
            return;
        }
        self.windows_order.push(handle).ok();
        if self.windows_order.len() == 1 {
            self.current_index = 0;
            self.apply_active_triplet(wm).await;
        } else {
            // Keep existing active set; new windows will get resources when they enter triplet
        }
        debug!("Registered window {:?}", handle);
    }

    /// Compute current/prev/next handles from order.
    fn current_prev_next(&self) -> Option<(WindowHandle, WindowHandle, WindowHandle)> {
        let n = self.windows_order.len();
        if n == 0 { return None; }
        let cur = self.windows_order[self.current_index];
        let prev = self.windows_order[(self.current_index + n - 1) % n];
        let next = self.windows_order[(self.current_index + 1) % n];
        Some((cur, prev, next))
    }

    /// Apply active triplet to WindowManager (allocate resources accordingly).
    async fn apply_active_triplet(&mut self, wm: &mut WindowManager) {
        if let Some((cur, prev, next)) = self.current_prev_next() {
            if let Some(service) = self.display_service {
                let fmt = service.pixel_format();
                wm.set_active_windows_with_format(&[prev, cur, next], fmt).await;
            } else {
                wm.set_active_windows(&[prev, cur, next]).await;
            }
            // Already set by set_active_windows_with_format
        }
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

    /// Process all pending redraws and update display with optimized performance.
    /// 
    /// This method implements several performance optimizations:
    /// - Early exit when no redraws are pending
    /// - Intelligent partial vs full screen update decisions
    /// - Optimized dirty region processing
    /// - Minimal memory allocations and copies
    pub async fn process_redraws(&mut self, wm: &mut WindowManager) {
        if self.pending_redraws.is_empty() {
            return;
        }

        let render_start = Instant::now();

        if let Some(service) = self.display_service {
            // Allocate working buffer for composition using service parameters
            let width = service.width();
            let height = service.height();
            let fb_size = service.framebuffer_size(width, height);
            let mut frame_buffer = vec![0u8; fb_size];
            let mut composite_canvas = Canvas::new(
                width,
                height
            );
            composite_canvas.set_resources(&mut frame_buffer);

            let dirty_regions = if let Some((cur, _prev, _next)) = self.current_prev_next() {
                self.collect_dirty_regions(wm, cur)
            } else { heapless::Vec::new() };
            debug!("Dirty regions count: {}", dirty_regions.len());

            if !dirty_regions.is_empty() {
                // Compose final frame only when there are dirty regions
                self.compose_frame_optimized(wm, &mut composite_canvas).await;

                // Optimized decision logic for partial vs full updates
                let update_strategy = self.determine_update_strategy(&dirty_regions);

                match update_strategy {
                    UpdateStrategy::FullScreen => {
                        debug!("Using full screen update ({} regions, {} area)", 
                               dirty_regions.len(), self.calculate_total_dirty_area(&dirty_regions));
                        service.draw_full(composite_canvas.buffer()).await;
                    }
                    UpdateStrategy::Partial(regions) => {
                        debug!("Using partial update ({} regions)", regions.len());
                        let bpp = service.pixel_format().bytes_per_pixel();
                        for region in regions.iter() {
                            // Extract region-sized buffer from composite canvas
                            let region_buffer = extract_region_buffer(
                                composite_canvas.buffer(),
                                region,
                                width,
                                height,
                                bpp
                            );
                            service.draw_region(&region_buffer, *region).await;
                        }
                    }
                }
                if let Some((cur, _p, _n)) = self.current_prev_next() {
                    self.clear_window_dirty_regions(wm, cur);
                }
            } else {
                // No dirty regions - no display update needed
                debug!("No dirty regions, skipping display update");
            }
        }

        self.pending_redraws.clear();
        debug!("Frame rendered in {} μs", render_start.elapsed().as_micros());
    }

    /// Get handle of currently focused window
    pub fn focused_window_handle(&self) -> Option<WindowHandle> {
        self.current_prev_next().map(|(cur, _, _)| cur)
    }

    /// Switch to next window with smooth animation
    pub async fn animate_to_next_window(&mut self, wm: &mut WindowManager) {
        self.animate_window_transition(wm, TransitionDirection::Next).await;
    }

    /// Switch to previous window with smooth animation
    pub async fn animate_to_previous_window(&mut self, wm: &mut WindowManager) {
        self.animate_window_transition(wm, TransitionDirection::Previous).await;
    }

    /// Execute smooth animated transition between windows
    async fn animate_window_transition(&mut self, wm: &mut WindowManager, direction: TransitionDirection) {
        let n = self.windows_order.len();
        if n < 2 { return; }

        let animation = SlideZoomAnimation::default();
        let src_idx = self.current_index;
        let dst_idx = self.calculate_transition_target_index(direction);

        debug!("Animating {} from window {} to {}",
               animation.name(), src_idx, dst_idx);

        // Update focus before animation so input focus aligns with motion intent
        self.current_index = dst_idx;

        // Compute handles now that index is updated
        let (cur, prev, next) = self.current_prev_next().unwrap();

        // Ensure active windows have resources
        self.apply_active_triplet(wm).await;

        // Execute smooth animation
        self.execute_animation(wm, &animation, direction).await;
    }

    /// Execute the actual animation frames
    async fn execute_animation(
        &mut self,
        wm: &mut WindowManager,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
    ) {
        // Prefer DisplayService for dynamic buffer sizing
        if let Some(service) = self.display_service {
            let width = service.width();
            let height = service.height();
            let fb_size = service.framebuffer_size(width, height);
            let mut composition_buffer: AllocVec<u8> = vec![0u8; fb_size];
            let mut canvas = Canvas::new(width, height);
            canvas.set_resources(&mut composition_buffer);

            let (cur, prev, next) = self.current_prev_next().unwrap();
            // Determine source and target by direction
            let (source_h, target_h) = match direction {
                TransitionDirection::Previous => (next, cur),
                TransitionDirection::Next => (prev, cur),
            };

            // Animate through all steps
            for step in 0..=self.animation_config.steps {
                let frame_timer = Instant::now();
                let progress = step as f32 / self.animation_config.steps as f32;
                let eased_progress = (self.animation_config.easing_fn)(progress);

                let frame = animation.animate_frame(eased_progress, direction);

                // Clear and compose frame
                canvas.clear_rgb(Rgb565::BLACK);
                // Blit source window (contained within closure to keep borrows local)
                let _ = wm.with_canvas(source_h, |src| {
                    let bpp = service.pixel_format().bytes_per_pixel();
                    CanvasBlitter::copy_full(&mut canvas, src, frame.source_x, frame.source_y, bpp);
                });
                // Blit target window
                let _ = wm.with_canvas(target_h, |dst| {
                    let bpp = service.pixel_format().bytes_per_pixel();
                    CanvasBlitter::copy_full(&mut canvas, dst, frame.target_x, frame.target_y, bpp);
                });

                // Display frame via service
                service.draw_full(canvas.buffer()).await;

                debug!("Animation step {}: {} μs", step, frame_timer.elapsed().as_micros());
                Timer::after(Duration::from_millis(self.animation_config.frame_delay_ms)).await;
            }
            return;
        }
    }

    // Helper methods
    fn calculate_transition_target_index(&self, direction: TransitionDirection) -> usize {
        let window_count = self.windows_order.len();
        match direction {
            TransitionDirection::Previous => {
                (self.current_index + window_count - 1) % window_count
            }
            TransitionDirection::Next => {
                (self.current_index + 1) % window_count
            }
        }
    }

    fn collect_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) -> heapless::Vec<Rect, 8> {
        let mut out = heapless::Vec::new();
        let _ = wm.with_canvas(handle, |canvas| {
            for r in canvas.dirty_regions() {
                debug!("Dirty region: {:?}", r);
                out.push(*r).ok();
            }
        });
        out
    }

    fn clear_window_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        let _ = wm.with_canvas(handle, |canvas| {
            canvas.flush();
        });
    }
    
    /// Poll for input events from specified window
    /// Check if window is currently focused
    pub fn is_window_focused(&self, handle: WindowHandle) -> bool {
        self.focused_window_handle().map(|h| h == handle).unwrap_or(false)
    }
    
    /// Optimized frame composition with performance improvements
    async fn compose_frame_optimized<'a>(&mut self, wm: &mut WindowManager, output_canvas: &mut Canvas<'a>) {
        // Clear to black background
        output_canvas.clear_rgb(Rgb565::BLACK);

        if self.windows_order.is_empty() {
            return;
        }

        if let Some((cur, _prev, _next)) = self.current_prev_next() {
            // Render single focused window with optimized dirty region handling
            let regions = self.collect_dirty_regions(wm, cur);
            if !regions.is_empty() {
                for r in regions.iter() {
                    let _ = wm.with_canvas(cur, |canvas| {
                        // When using service, prefer its bpp; else assume RGB565
                        let bpp = self.display_service.map(|s| s.pixel_format().bytes_per_pixel()).unwrap_or(2);
                        CanvasBlitter::copy_region(output_canvas, canvas, *r, 0, 0, bpp);
                    });
                }
            }
        }
    }
    
    /// Determine the optimal update strategy based on dirty regions
    fn determine_update_strategy(&self, dirty_regions: &heapless::Vec<Rect, 8>) -> UpdateStrategy {
        if dirty_regions.is_empty() {
            return UpdateStrategy::FullScreen;
        }
        
        let full_area = if let Some(service) = self.display_service {
            service.width() * service.height()
        } else {
            FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT
        };
        let total_area = self.calculate_total_dirty_area(dirty_regions);
        
        // Heuristics for update strategy:
        // - If too many regions (>6), prefer full screen
        // - If total dirty area > 33% of screen, prefer full screen
        // - Otherwise use partial updates
        if dirty_regions.len() > 6 || total_area * 3 > full_area {
            UpdateStrategy::FullScreen
        } else {
            let mut regions = heapless::Vec::new();
            for region in dirty_regions.iter() {
                regions.push(*region).ok();
            }
            UpdateStrategy::Partial(regions)
        }
    }
    
    /// Calculate total dirty area for optimization decisions
    fn calculate_total_dirty_area(&self, dirty_regions: &heapless::Vec<Rect, 8>) -> u32 {
        let mut total_area: u32 = 0;
        for r in dirty_regions.iter() {
            total_area = total_area.saturating_add(r.size.width.saturating_mul(r.size.height));
        }
        total_area
    }
}
/// Internal: SUI -> compositor command channel (e.g., navigation). Not limited to swipes.
pub static SUI_COMMAND_CH: Channel<CriticalSectionRawMutex, TransitionDirection, 4> = Channel::new();

/// Simple System UI input consumer: detects edge swipes for navigation.
#[embassy_executor::task]
pub async fn system_ui_consume_events() {
    use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

    const EDGE_THRESHOLD: i32 = 24; // px from left/right edge
    const MIN_SWIPE_DISTANCE: i32 = 60;
    const MIN_SWIPE_ANGLE_TOLERANCE: i32 = 20; // vertical drift tolerance in px

    let mut tracking = false;
    let mut start_x = 0i32;
    let mut start_y = 0i32;
    let mut from_left = false;
    let mut from_right = false;
    let mut fired = false;

    loop {
        let ev = SUI_EVENT_CH.receive().await;

        let mut consumed = false;
        if let HighLevelEvent::Motion(MotionEvent { action, pointers, .. }) = ev {
            if let Some(p) = pointers[0] {
                match action {
                    TouchAction::Down => {
                        tracking = false;
                        from_left = p.x <= EDGE_THRESHOLD;
                        from_right = p.x >= ((FRAME_BUFFER_WIDTH as i32) - EDGE_THRESHOLD);
                        if from_left || from_right {
                            tracking = true;
                            start_x = p.x;
                            start_y = p.y;
                        }
                    }
                    TouchAction::Move => {
                        if tracking && !fired {
                            let dx = p.x - start_x;
                            let dy = (p.y - start_y).abs();
                            // Ensure swipe is mostly horizontal and started at edge
                            if dy <= MIN_SWIPE_ANGLE_TOLERANCE {
                                if from_left && dx > MIN_SWIPE_DISTANCE {
                                    consumed = true;
                                    fired = true;
                                    // Swipe from left edge towards right -> navigate to next
                                    SUI_COMMAND_CH.send(TransitionDirection::Next).await;
                                } else if from_right && (-dx) > MIN_SWIPE_DISTANCE {
                                    consumed = true;
                                    fired = true;
                                    // Swipe from right edge towards left -> navigate to previous
                                    SUI_COMMAND_CH.send(TransitionDirection::Previous).await;
                                }
                            }
                        }
                    }
                    TouchAction::Up => {
                        tracking = false;
                        fired = false;
                    }
                }
            }
        }

        SUI_ACK_CH.send(consumed).await;
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

use micromath::F32Ext;

/// Extract a region-sized buffer from a full framebuffer
fn extract_region_buffer(
    full_buffer: &[u8],
    region: &Rect,
    full_width: u32,
    _full_height: u32,
    bytes_per_pixel: usize,
) -> AllocVec<u8> {
    let region_x = region.top_left.x as u32;
    let region_y = region.top_left.y as u32;
    let region_width = region.size.width as u32;
    let region_height = region.size.height as u32;
    
    let region_size = (region_width * region_height * bytes_per_pixel as u32) as usize;
    let mut region_buffer = vec![0u8; region_size];
    
    let full_bytes_per_row = full_width as usize * bytes_per_pixel;
    let region_bytes_per_row = region_width as usize * bytes_per_pixel;
    
    for row in 0..region_height as usize {
        let src_start = ((region_y as usize + row) * full_bytes_per_row) + (region_x as usize * bytes_per_pixel);
        let dst_start = row * region_bytes_per_row;
        
        region_buffer[dst_start..dst_start + region_bytes_per_row]
            .copy_from_slice(&full_buffer[src_start..src_start + region_bytes_per_row]);
    }
    
    region_buffer
}