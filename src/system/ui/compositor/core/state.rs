use alloc::vec::Vec;
use defmt::warn;

use crate::system::hal::display::PixelFormat;
use crate::system::services::display::Display;
use crate::system::ui::windowing::{WindowHandle, WindowManager};

use super::super::animation::AnimationConfig;

pub(super) const MAX_WINDOWS: usize = 8;
pub(super) const MAX_REDRAW_REQUESTS: usize = 4;

/// Growable framebuffer reused by the compositor to avoid per-frame heap churn.
pub(super) struct ScratchFrame {
    buffer: Vec<u8>,
}

impl ScratchFrame {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn acquire(&mut self, width: u32, height: u32, format: PixelFormat) -> &mut [u8] {
        let required = format.framebuffer_size(width, height);
        if self.buffer.len() < required {
            self.buffer.resize(required, 0);
        }
        let slice = &mut self.buffer[..required];
        slice.fill(0);
        slice
    }
}

#[derive(Clone, Copy)]
pub(super) struct TransitionSession {
    pub direction: super::super::animation::TransitionDirection,
    pub source: WindowHandle,
    pub target: WindowHandle,
}

/// Core compositor state machine. Keeps window ordering, active triplets, and display binding.
pub struct UICompositor {
    pub(super) windows_order: Vec<WindowHandle>,
    pub(super) current_index: usize,
    pub(super) display_service: Option<Display>,
    pub(super) pending_redraws: Vec<WindowHandle>,
    pub(super) neighbor_warmups: Vec<WindowHandle>,
    pub(super) animation_config: AnimationConfig,
    pub(super) active_transition: Option<TransitionSession>,
    pub(super) scratch: ScratchFrame,
}

impl UICompositor {
    pub fn new() -> Self {
        Self {
            windows_order: Vec::with_capacity(MAX_WINDOWS),
            current_index: 0,
            display_service: None,
            pending_redraws: Vec::with_capacity(MAX_REDRAW_REQUESTS),
            neighbor_warmups: Vec::with_capacity(MAX_WINDOWS),
            animation_config: AnimationConfig::default(),
            active_transition: None,
            scratch: ScratchFrame::new(),
        }
    }

    pub fn attach_display_service(&mut self, service: Display) {
        self.display_service = Some(service);
    }

    pub fn set_animation_config(&mut self, config: AnimationConfig) {
        self.animation_config = config;
    }

    pub async fn register_window(&mut self, wm: &mut WindowManager, handle: WindowHandle) {
        if self.windows_order.len() >= MAX_WINDOWS {
            warn!("Compositor window order full");
            return;
        }
        self.windows_order.push(handle);
        if self.windows_order.len() == 1 {
            self.current_index = 0;
            self.apply_active_triplet(wm).await;
        }
    }

    pub(super) fn current_prev_next(&self) -> Option<(WindowHandle, WindowHandle, WindowHandle)> {
        let count = self.windows_order.len();
        if count == 0 {
            return None;
        }
        let cur = self.windows_order[self.current_index];
        let prev = self.windows_order[(self.current_index + count - 1) % count];
        let next = self.windows_order[(self.current_index + 1) % count];
        Some((cur, prev, next))
    }

    pub(super) async fn apply_active_triplet(&mut self, wm: &mut WindowManager) {
        if let Some((cur, prev, next)) = self.current_prev_next() {
            wm.set_active_windows(&[prev, cur, next]).await;
        }
    }

    pub fn request_redraw(&mut self, window_handle: WindowHandle) {
        if !self.pending_redraws.contains(&window_handle) {
            if self.pending_redraws.len() < MAX_REDRAW_REQUESTS {
                self.pending_redraws.push(window_handle);
            }
        }
    }

    pub fn focused_window_handle(&self) -> Option<WindowHandle> {
        if self.transition_in_progress() {
            return None;
        }
        self.current_prev_next().map(|(current, _, _)| current)
    }

    pub fn is_window_focused(&self, handle: WindowHandle) -> bool {
        self.focused_window_handle()
            .map(|focused| focused == handle)
            .unwrap_or(false)
    }

    pub fn display_dimensions(&self) -> Option<(u32, u32)> {
        self.display_service
            .as_ref()
            .map(|display| (display.width(), display.height()))
    }

    pub(super) fn transition_in_progress(&self) -> bool {
        self.active_transition.is_some()
    }

    pub(super) fn queue_neighbor_warmups(&mut self) {
        if let Some((current, prev, next)) = self.current_prev_next() {
            if prev != current {
                self.enqueue_warmup(prev);
            }
            if next != current {
                self.enqueue_warmup(next);
            }
        }
    }

    pub fn take_warmup_request(&mut self, handle: WindowHandle) -> bool {
        if let Some(index) = self
            .neighbor_warmups
            .iter()
            .position(|queued| queued == &handle)
        {
            self.neighbor_warmups.swap_remove(index);
            true
        } else {
            false
        }
    }

    fn enqueue_warmup(&mut self, handle: WindowHandle) {
        if self.neighbor_warmups.iter().any(|queued| queued == &handle) {
            return;
        }
        if self.neighbor_warmups.len() < MAX_WINDOWS {
            self.neighbor_warmups.push(handle);
        }
    }
}
