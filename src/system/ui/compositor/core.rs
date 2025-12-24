//! Core compositor: window ordering, composition pipeline, redraw processing, and animations.
//!
//! Invariants:
//! - All active windows' `DrawingSurface`s share the same negotiated `PixelFormat`.
//! - The compositor never performs color conversion; it composes buffers with identical BPP.
//! - `bytes_per_pixel()` is queried from `DrawingSurface` to remain format-agnostic.

use alloc::vec::Vec as AllocVec;
use defmt::{debug, warn};
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;

use super::animation::{AnimationConfig, SlideZoomAnimation, TransitionDirection, WindowAnimation};
use super::blitter::SurfaceBlitter;
use super::region::extract_region_buffer;
use super::strategy::UpdateStrategy;
use crate::libs::gfx::color::Rgba8888;
use crate::system::hal::display::PixelFormat;
use crate::system::ui::display::Display;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::system::ui::window::WindowHandle;
use crate::system::ui::window_manager::WindowManager;
use crate::util::math::primitives::Rect;

const MAX_WINDOWS: usize = 8;
const MAX_REDRAW_REQUESTS: usize = 4;

/// Growable framebuffer reused by the compositor to avoid per-frame heap churn.
struct ScratchFrame {
    buffer: AllocVec<u8>,
}

impl ScratchFrame {
    fn new() -> Self {
        Self {
            buffer: AllocVec::new(),
        }
    }

    fn acquire(&mut self, width: u32, height: u32, format: PixelFormat) -> &mut [u8] {
        let required = (width as usize) * (height as usize) * format.bytes_per_pixel();
        if self.buffer.len() < required {
            self.buffer.resize(required, 0);
        }
        let slice = &mut self.buffer[..required];
        slice.fill(0);
        slice
    }
}

#[derive(Clone, Copy)]
struct TransitionSession {
    direction: TransitionDirection,
    source: WindowHandle,
    target: WindowHandle,
}

// TransitionDirection is defined in animation module; use that single source of truth

/// Space-grade UI compositor focused on composition and transitions.
///
/// Responsibilities:
/// - Maintain logical window order and focus
/// - Request resources for the active triplet (prev, current, next)
/// - Compose frames using format-agnostic blits based on surface BPP
/// - Decide between full-screen and partial updates
/// - Run smooth, configurable animations between windows
pub struct UICompositor {
    windows_order: heapless::Vec<WindowHandle, MAX_WINDOWS>,
    current_index: usize,
    display_service: Option<Display>,
    pending_redraws: heapless::Vec<WindowHandle, MAX_REDRAW_REQUESTS>,
    animation_config: AnimationConfig,
    active_transition: Option<TransitionSession>,
    scratch: ScratchFrame,
}

impl UICompositor {
    /// Create a new compositor instance.
    pub fn new() -> Self {
        Self {
            windows_order: heapless::Vec::new(),
            current_index: 0,
            display_service: None,
            pending_redraws: heapless::Vec::new(),
            animation_config: AnimationConfig::default(),
            active_transition: None,
            scratch: ScratchFrame::new(),
        }
    }

    /// Attach negotiated display facade for sizing and presentation.
    pub fn attach_display_service(&mut self, service: Display) {
        self.display_service = Some(service);
    }

    /// Update animation parameters.
    pub fn set_animation_config(&mut self, config: AnimationConfig) {
        self.animation_config = config;
    }

    pub async fn register_window(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        if self.windows_order.len() >= MAX_WINDOWS {
            warn!("Compositor window order full");
            return;
        }
        self.windows_order.push(handle).ok();
        if self.windows_order.len() == 1 {
            self.current_index = 0;
            self.apply_active_triplet(wm).await;
        }
    }

    fn current_prev_next(&self) -> Option<(WindowHandle, WindowHandle, WindowHandle)> {
        let n = self.windows_order.len();
        if n == 0 {
            return None;
        }
        let cur = self.windows_order[self.current_index];
        let prev = self.windows_order[(self.current_index + n - 1) % n];
        let next = self.windows_order[(self.current_index + 1) % n];
        Some((cur, prev, next))
    }

    /// Ensure resources are allocated for current/prev/next windows.
    async fn apply_active_triplet(&mut self, wm: &mut WindowManager) {
        if let Some((cur, prev, next)) = self.current_prev_next() {
            wm.set_active_windows(&[prev, cur, next]).await;
        }
    }

    pub fn request_redraw(&mut self, window_handle: WindowHandle) {
        if !self.pending_redraws.contains(&window_handle) {
            let _ = self.pending_redraws.push(window_handle);
        }
    }

    /// Process pending redraws; compose and present with an optimized strategy.
    pub async fn process_redraws(&mut self, wm: &mut WindowManager) {
        if self.pending_redraws.is_empty() {
            return;
        }
        if self.transition_in_progress() {
            return;
        }
        let render_start = Instant::now();

        let Some((width, height, pixel_format)) = self
            .display_service
            .as_ref()
            .map(|display| (display.width(), display.height(), display.pixel_format()))
        else {
            return;
        };
        let frame_area = width * height;

        let mut dirty_regions = heapless::Vec::<Rect, 8>::new();
        let mut active_window = None;
        if let Some((cur, _prev, _next)) = self.current_prev_next() {
            dirty_regions = self.collect_dirty_regions(wm, cur);
            if !dirty_regions.is_empty() {
                active_window = Some(cur);
            }
        }

        if let Some(source_handle) = active_window {
            let scratch = self.scratch.acquire(width, height, pixel_format);
            let mut composite_surface = DrawingSurface::new_unattached(width, height, pixel_format);
            composite_surface.attach_buffer(scratch);
            composite_surface.clear(Rgba8888::rgba(0, 0, 0, 255));

            Self::blit_window_regions(wm, &mut composite_surface, source_handle, &dirty_regions);

            {
                let Some(display) = self.display_service.as_ref() else {
                    return;
                };
                match super::strategy::determine_update_strategy(frame_area, &dirty_regions) {
                    UpdateStrategy::FullScreen => {
                        display.draw_full(composite_surface.buffer()).await;
                    }
                    UpdateStrategy::Partial(regions) => {
                        for region in regions.iter() {
                            let region_buffer = extract_region_buffer(
                                composite_surface.buffer(),
                                region,
                                width,
                                height,
                                composite_surface.bytes_per_pixel(),
                            );
                            display.draw_region(&region_buffer, *region).await;
                        }
                    }
                }
            }

            drop(composite_surface);
            self.clear_window_dirty_regions(wm, source_handle);
        }

        self.pending_redraws.clear();
        debug!(
            "Frame rendered in {} μs",
            render_start.elapsed().as_micros()
        );
    }

    pub fn focused_window_handle(&self) -> Option<WindowHandle> {
        if self.transition_in_progress() {
            return None;
        }
        self.current_prev_next().map(|(c, _, _)| c)
    }
    pub fn is_window_focused(&self, handle: WindowHandle) -> bool {
        self.focused_window_handle()
            .map(|h| h == handle)
            .unwrap_or(false)
    }

    /// Returns display width/height if a display is attached.
    pub fn display_dimensions(&self) -> Option<(u32, u32)> {
        self.display_service
            .as_ref()
            .map(|d| (d.width(), d.height()))
    }

    pub async fn animate_to_next_window(&mut self, wm: &mut WindowManager) {
        self.commit_transition(wm, TransitionDirection::Next, 0.0)
            .await;
    }
    pub async fn animate_to_previous_window(&mut self, wm: &mut WindowManager) {
        self.commit_transition(wm, TransitionDirection::Previous, 0.0)
            .await;
    }

    /// Render a snapshot of the transition at the supplied progress (0.0..=1.0).
    pub async fn preview_transition(
        &mut self,
        wm: &mut WindowManager,
        direction: TransitionDirection,
        progress: f32,
    ) {
        let Some((source, target, _)) = self.transition_context(direction) else {
            return;
        };
        if !self.ensure_transition_surfaces(wm, source, target).await {
            return;
        }
        self.begin_transition_session(direction, source, target);
        let animation = SlideZoomAnimation::default();
        self.render_transition_frame(
            wm,
            &animation,
            direction,
            progress.clamp(0.0, 1.0),
            source,
            target,
        )
        .await;
    }

    /// Complete the transition, animating from the provided starting progress to 1.0.
    pub async fn commit_transition(
        &mut self,
        wm: &mut WindowManager,
        direction: TransitionDirection,
        start_progress: f32,
    ) {
        let Some((source, target, dst_idx)) = self.transition_context(direction) else {
            return;
        };
        if !self.ensure_transition_surfaces(wm, source, target).await {
            return;
        }
        self.begin_transition_session(direction, source, target);
        let animation = SlideZoomAnimation::default();
        self.run_transition_animation(
            wm,
            &animation,
            direction,
            source,
            target,
            start_progress,
            1.0,
        )
        .await;

        self.current_index = dst_idx;
        self.apply_active_triplet(wm).await;
        self.request_redraw(target);
        self.end_transition_session();
    }

    /// Revert an in-progress transition by animating back to the resting state.
    pub async fn cancel_transition(
        &mut self,
        wm: &mut WindowManager,
        direction: TransitionDirection,
        start_progress: f32,
    ) {
        let Some((source, target, _)) = self.transition_context(direction) else {
            return;
        };
        if !self.ensure_transition_surfaces(wm, source, target).await {
            return;
        }
        self.begin_transition_session(direction, source, target);
        let animation = SlideZoomAnimation::default();
        self.run_transition_animation(
            wm,
            &animation,
            direction,
            source,
            target,
            start_progress,
            0.0,
        )
        .await;

        if let Some((current, _prev, _next)) = self.current_prev_next() {
            self.request_redraw(current);
        }
        self.end_transition_session();
    }

    async fn ensure_transition_surfaces(
        &mut self,
        wm: &mut WindowManager,
        source: WindowHandle,
        target: WindowHandle,
    ) -> bool {
        let source_ready = wm.is_active(source);
        let target_ready = wm.is_active(target);
        if source_ready && target_ready {
            return true;
        }

        self.apply_active_triplet(wm).await;

        let post_source_ready = wm.is_active(source);
        let post_target_ready = wm.is_active(target);
        if !(post_source_ready && post_target_ready) {
            warn!("Transition surfaces missing resources");
        }
        post_source_ready && post_target_ready
    }

    fn begin_transition_session(
        &mut self,
        direction: TransitionDirection,
        source: WindowHandle,
        target: WindowHandle,
    ) {
        self.active_transition = Some(TransitionSession {
            direction,
            source,
            target,
        });
    }

    fn end_transition_session(&mut self) {
        self.active_transition = None;
    }

    fn transition_in_progress(&self) -> bool {
        self.active_transition.is_some()
    }
    fn transition_context(
        &self,
        direction: TransitionDirection,
    ) -> Option<(WindowHandle, WindowHandle, usize)> {
        if self.windows_order.len() < 2 {
            return None;
        }
        let (cur, prev, next) = self.current_prev_next()?;
        let n = self.windows_order.len();
        match direction {
            TransitionDirection::Next => {
                let dst_idx = (self.current_index + 1) % n;
                Some((cur, next, dst_idx))
            }
            TransitionDirection::Previous => {
                let dst_idx = (self.current_index + n - 1) % n;
                Some((cur, prev, dst_idx))
            }
        }
    }

    async fn render_transition_frame(
        &mut self,
        wm: &mut WindowManager,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
        progress: f32,
        source: WindowHandle,
        target: WindowHandle,
    ) {
        let display = match self.display_service.as_ref() {
            Some(service) => service,
            None => return,
        };
        let width = display.width();
        let height = display.height();
        let pixel_format = display.pixel_format();

        let scratch = self.scratch.acquire(width, height, pixel_format);
        let mut surface = DrawingSurface::new_unattached(width, height, pixel_format);
        surface.attach_buffer(scratch);
        surface.clear(Rgba8888::rgba(0, 0, 0, 255));

        Self::compose_transition_frame(
            wm,
            &mut surface,
            animation,
            direction,
            progress,
            width,
            source,
            target,
        );

        display.draw_full(surface.buffer()).await;
    }

    async fn run_transition_animation(
        &mut self,
        wm: &mut WindowManager,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
        source: WindowHandle,
        target: WindowHandle,
        start_progress: f32,
        end_progress: f32,
    ) {
        let display = match self.display_service.as_ref() {
            Some(service) => service,
            None => return,
        };
        let width = display.width();
        let height = display.height();
        let pixel_format = display.pixel_format();

        let steps = self.animation_config.steps.max(1) as i32;
        let frame_delay = self.animation_config.frame_delay_ms;
        let easing = self.animation_config.easing_fn;

        let scratch = self.scratch.acquire(width, height, pixel_format);
        let mut surface = DrawingSurface::new_unattached(width, height, pixel_format);
        surface.attach_buffer(scratch);
        surface.clear(Rgba8888::rgba(0, 0, 0, 255));
        let mut start_idx = ((start_progress.clamp(0.0, 1.0)) * steps as f32).round() as i32;
        let mut end_idx = ((end_progress.clamp(0.0, 1.0)) * steps as f32).round() as i32;
        start_idx = start_idx.clamp(0, steps);
        end_idx = end_idx.clamp(0, steps);
        let step_delta = if start_idx <= end_idx { 1 } else { -1 };
        let mut current_idx = start_idx;

        loop {
            let normalized = current_idx as f32 / steps as f32;
            let eased = easing(normalized);
            Self::compose_transition_frame(
                wm,
                &mut surface,
                animation,
                direction,
                eased,
                width,
                source,
                target,
            );
            display.draw_full(surface.buffer()).await;

            if current_idx == end_idx {
                break;
            }

            Timer::after(Duration::from_millis(frame_delay)).await;
            current_idx += step_delta;
        }
    }

    fn compose_transition_frame<'a>(
        wm: &mut WindowManager,
        surface: &mut DrawingSurface<'a>,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
        progress: f32,
        screen_width: u32,
        source: WindowHandle,
        target: WindowHandle,
    ) {
        surface.clear(Rgba8888::rgba(0, 0, 0, 255));
        let frame = animation.animate_frame(progress, direction, screen_width);
        let _ = wm.with_surface(source, |src| {
            SurfaceBlitter::copy_full(surface, src, frame.source_x, frame.source_y);
        });
        let _ = wm.with_surface(target, |dst| {
            SurfaceBlitter::copy_full(surface, dst, frame.target_x, frame.target_y);
        });
    }

    fn collect_dirty_regions(
        &mut self,
        wm: &mut WindowManager,
        handle: WindowHandle,
    ) -> heapless::Vec<Rect, 8> {
        let mut out = heapless::Vec::new();
        let _ = wm.with_surface(handle, |surface| {
            for r in surface.dirty_regions() {
                let _ = out.push(*r);
            }
        });
        out
    }

    fn clear_window_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        let _ = wm.with_surface(handle, |surface| {
            surface.flush();
        });
    }
    fn blit_window_regions<'a>(
        wm: &mut WindowManager,
        output_surface: &mut DrawingSurface<'a>,
        source: WindowHandle,
        regions: &[Rect],
    ) {
        if regions.is_empty() {
            return;
        }
        let _ = wm.with_surface(source, |surface| {
            for region in regions {
                SurfaceBlitter::copy_region(output_surface, surface, *region, 0, 0);
            }
        });
    }
}
