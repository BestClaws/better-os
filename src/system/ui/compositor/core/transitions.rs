use defmt::warn;
use embassy_time::{Duration, Timer};
use micromath::F32Ext;

use super::super::animation::{SlideZoomAnimation, TransitionDirection, WindowAnimation};
use super::super::blitter::SurfaceBlitter;
use super::state::{TransitionSession, UICompositor};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::system::ui::windowing::{WindowHandle, WindowManager};
use rust_gfx::color::Rgba8888;

impl UICompositor {
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

    fn transition_context(
        &self,
        direction: TransitionDirection,
    ) -> Option<(WindowHandle, WindowHandle, usize)> {
        if self.windows_order.len() < 2 {
            return None;
        }
        let (current, previous, next) = self.current_prev_next()?;
        let count = self.windows_order.len();

        match direction {
            TransitionDirection::Next => {
                let dst_idx = (self.current_index + 1) % count;
                Some((current, next, dst_idx))
            }
            TransitionDirection::Previous => {
                let dst_idx = (self.current_index + count - 1) % count;
                Some((current, previous, dst_idx))
            }
        }
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

    async fn render_transition_frame(
        &mut self,
        wm: &mut WindowManager,
        animation: &dyn WindowAnimation,
        direction: TransitionDirection,
        progress: f32,
        source: WindowHandle,
        target: WindowHandle,
    ) {
        let Some(display) = self.display_service.as_ref() else {
            return;
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
        let Some(display) = self.display_service.as_ref() else {
            return;
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

    fn compose_transition_frame(
        wm: &mut WindowManager,
        surface: &mut DrawingSurface<'_>,
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
}
