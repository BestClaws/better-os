//! Core compositor: window ordering, composition pipeline, redraw processing, and animations.
//!
//! Invariants:
//! - All active windows' `DrawingSurface`s share the same negotiated `PixelFormat`.
//! - The compositor never performs color conversion; it composes buffers with identical BPP.
//! - `bytes_per_pixel()` is queried from `DrawingSurface` to remain format-agnostic.

use alloc::vec;
use alloc::vec::Vec as AllocVec;
use defmt::{debug, warn};
use embassy_time::{Duration, Instant, Timer};

use crate::libs::gfx::two_d::{Rect, Rgba8888};
use crate::system::ui::display::Display;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::system::ui::window::WindowHandle;
use crate::system::ui::window_manager::WindowManager;

use super::animation::{AnimationConfig, WindowAnimation, SlideZoomAnimation, TransitionDirection};
use super::blitter::SurfaceBlitter;
use super::strategy::UpdateStrategy;
use super::region::extract_region_buffer;

const MAX_WINDOWS: usize = 8;
const MAX_REDRAW_REQUESTS: usize = 4;

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
    display_service: Option<&'static Display>,
    pending_redraws: heapless::Vec<WindowHandle, MAX_REDRAW_REQUESTS>,
    animation_config: AnimationConfig,
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
        }
    }

    /// Attach negotiated display facade for sizing and presentation.
    pub fn attach_display_service(&mut self, service: &'static Display) {
        self.display_service = Some(service);
    }

    /// Update animation parameters.
    pub fn set_animation_config(&mut self, config: AnimationConfig) { self.animation_config = config; }

    pub async fn register_window(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        if self.windows_order.len() >= MAX_WINDOWS { warn!("Compositor window order full"); return; }
        self.windows_order.push(handle).ok();
        if self.windows_order.len() == 1 {
            self.current_index = 0;
            self.apply_active_triplet(wm).await;
        }
    }

    fn current_prev_next(&self) -> Option<(WindowHandle, WindowHandle, WindowHandle)> {
        let n = self.windows_order.len();
        if n == 0 { return None; }
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
        if self.pending_redraws.is_empty() { return; }
        let render_start = Instant::now();

        if let Some(service) = self.display_service {
            let width = service.width();
            let height = service.height();
            let fb_size = service.framebuffer_size(width, height);
            let mut frame_buffer = vec![0u8; fb_size];
            let mut composite_surface = DrawingSurface::new_unattached(width, height, service.pixel_format());
            composite_surface.attach_buffer(&mut frame_buffer);

            let dirty_regions = if let Some((cur, _prev, _next)) = self.current_prev_next() {
                self.collect_dirty_regions(wm, cur)
            } else { heapless::Vec::new() };

            if !dirty_regions.is_empty() {
                self.compose_frame_optimized(wm, &mut composite_surface).await;

                match super::strategy::determine_update_strategy(self.display_service, &dirty_regions) {
                    UpdateStrategy::FullScreen => {
                        service.draw_full(composite_surface.buffer()).await;
                    }
                    UpdateStrategy::Partial(regions) => {
                        for region in regions.iter() {
                            let region_buffer = extract_region_buffer(
                                composite_surface.buffer(), region, width, height, composite_surface.bytes_per_pixel());
                            service.draw_region(&region_buffer, *region).await;
                        }
                    }
                }
                if let Some((cur, _p, _n)) = self.current_prev_next() { self.clear_window_dirty_regions(wm, cur); }
            }
        }

        self.pending_redraws.clear();
        debug!("Frame rendered in {} μs", render_start.elapsed().as_micros());
    }

    pub fn focused_window_handle(&self) -> Option<WindowHandle> { self.current_prev_next().map(|(c,_,_)| c) }
    pub fn is_window_focused(&self, handle: WindowHandle) -> bool { self.focused_window_handle().map(|h| h == handle).unwrap_or(false) }

    pub async fn animate_to_next_window(&mut self, wm: &mut WindowManager) { self.animate_window_transition(wm, TransitionDirection::Next).await; }
    pub async fn animate_to_previous_window(&mut self, wm: &mut WindowManager) { self.animate_window_transition(wm, TransitionDirection::Previous).await; }

    async fn animate_window_transition(&mut self, wm: &mut WindowManager, direction: TransitionDirection) {
        let n = self.windows_order.len(); if n < 2 { return; }
        let animation = SlideZoomAnimation::default();
        let dst_idx = match direction { TransitionDirection::Previous => (self.current_index + n - 1) % n, TransitionDirection::Next => (self.current_index + 1) % n };
        self.current_index = dst_idx;
        self.apply_active_triplet(wm).await;
        self.execute_animation(wm, &animation, direction).await;
    }

    async fn execute_animation(
        &mut self,
        wm: &mut WindowManager,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
    ) {
        if let Some(service) = self.display_service {
            let width = service.width();
            let height = service.height();
            let fb_size = service.framebuffer_size(width, height);
            let mut composition_buffer: AllocVec<u8> = vec![0u8; fb_size];
            let mut surface = DrawingSurface::new_unattached(width, height, service.pixel_format());
            surface.attach_buffer(&mut composition_buffer);

            let (cur, prev, next) = self.current_prev_next().unwrap();
            let (source_h, target_h) = match direction { TransitionDirection::Previous => (next, cur), TransitionDirection::Next => (prev, cur) };

            for step in 0..=self.animation_config.steps {
                let progress = step as f32 / self.animation_config.steps as f32;
                let eased_progress = (self.animation_config.easing_fn)(progress);
                let frame = animation.animate_frame(eased_progress, direction, width);

                surface.clear(Rgba8888::opaque(0,0,0));
                let _ = wm.with_surface(source_h, |src| {
                    SurfaceBlitter::copy_full(&mut surface, src, frame.source_x, frame.source_y);
                });
                let _ = wm.with_surface(target_h, |dst| {
                    SurfaceBlitter::copy_full(&mut surface, dst, frame.target_x, frame.target_y);
                });

                service.draw_full(surface.buffer()).await;
                Timer::after(Duration::from_millis(self.animation_config.frame_delay_ms)).await;
            }
        }
    }

    fn collect_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) -> heapless::Vec<Rect, 8> {
        let mut out = heapless::Vec::new();
        let _ = wm.with_surface(handle, |surface| { for r in surface.dirty_regions() { let _ = out.push(*r); } });
        out
    }

    fn clear_window_dirty_regions(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        let _ = wm.with_surface(handle, |surface| { surface.flush(); });
    }

    async fn compose_frame_optimized<'a>(&mut self, wm: &mut WindowManager, output_surface: &mut DrawingSurface<'a>) {
        output_surface.clear(Rgba8888::opaque(0,0,0));
        if self.windows_order.is_empty() { return; }
        if let Some((cur, _prev, _next)) = self.current_prev_next() {
            let regions = self.collect_dirty_regions(wm, cur);
            if !regions.is_empty() {
                for r in regions.iter() {
                    let _ = wm.with_surface(cur, |surface| {
                        SurfaceBlitter::copy_region(output_surface, surface, *r, 0, 0);
                    });
                }
            }
        }
    }
}

