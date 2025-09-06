use defmt::{debug, warn};

use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::window::{Window, WindowHandle};
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::resources::input_channels::INPUT_CHANNEL_POOL;

/// Maximum supported windows in the system. Matches former compositor limit.
const MAX_WINDOWS: usize = 8;

/// Manages windows and their scarce resources (framebuffers and input channels).
///
/// Responsibilities:
/// - Create and destroy windows
/// - Allocate/reclaim resources for a chosen active set (up to 3 windows)
/// - Provide access to canvases and input channels
/// - Track one-shot initial paint requests for pre-active (next/previous) windows
pub struct WindowManager {
    windows: heapless::Vec<Window, MAX_WINDOWS>,
}

impl WindowManager {
    /// Create a new empty window manager.
    pub fn new() -> Self {
        Self { windows: heapless::Vec::new() }
    }

    /// Create a new window and add it to the manager.
    pub async fn create_window(&mut self, width: u32, height: u32, id: usize) -> Option<WindowHandle> {
        if self.windows.len() >= MAX_WINDOWS {
            warn!("WindowManager capacity reached; cannot create more windows");
            return None;
        }

        let window = Window::new(width, height, id).await;
        let handle = window.handle();
        self.windows.push(window).ok()?;
        debug!("Window created: id={}, size={}x{}", id, width, height);
        Some(handle)
    }

    /// Destroy a window and reclaim any resources held by it.
    pub fn destroy_window(&mut self, handle: WindowHandle) -> bool {
        if let Some(idx) = self.index_of(handle) {
            let mut window = self.windows.swap_remove(idx);
            if window.framebuffer_id().is_some() {
                // Ensure resources are returned
                window.relax();
            }
            debug!("Window destroyed: {:?}", handle);
            true
        } else {
            false
        }
    }

    /// Set the active windows. Active windows have framebuffer and input-channel resources.
    /// All windows not in the set will have their resources reclaimed.
    pub async fn set_active_windows(&mut self, active: &[WindowHandle]) {
        // Reclaim resources from windows not in the active set
        for window in self.windows.iter_mut() {
            let is_active = active.iter().any(|h| h == &window.handle());
            if !is_active && window.framebuffer_id().is_some() {
                debug!("Releasing resources for inactive window {:?}", window.handle());
                window.relax();
            }
        }

        // Allocate resources for active windows
        for handle in active.iter() {
            if let Some(idx) = self.index_of(*handle) {
                let window = &mut self.windows[idx];
                if window.framebuffer_id().is_none() {
                    debug!("Allocating resources for active window {:?}", handle);
                    match FRAMEBUFFER_POOL.allocate().await {
                        Some(fb) => {
                            match INPUT_CHANNEL_POOL.allocate().await {
                                Some(ic) => {
                                    window.set_resources(fb, ic).await;
                                }
                                None => {
                                    warn!("Input channel allocation failed for window {:?}", handle);
                                    FRAMEBUFFER_POOL.release(&fb);
                                }
                            }
                        }
                        None => {
                            warn!("Framebuffer allocation failed for window {:?}", handle);
                        }
                    }
                }
            }
        }
    }

    /// Check if the window currently holds resources.
    pub fn is_active(&self, handle: WindowHandle) -> bool {
        self.get_window(handle)
            .map(|w| w.framebuffer_id().is_some())
            .unwrap_or(false)
    }

    /// Provide mutable access to a window's canvas for drawing.
    pub fn with_canvas<R>(&mut self, handle: WindowHandle, f: impl FnOnce(&mut Canvas) -> R) -> Option<R> {
        let window = self.get_window_mut(handle)?;
        let canvas_opt = window.canvas();
        if let Some(canvas) = canvas_opt.as_mut() {
            Some(f(canvas))
        } else {
            None
        }
    }

    /// Get a single pending input event for the window, if any.
    pub fn poll_window_input(&mut self, handle: WindowHandle) -> Option<HumanInputEvent> {
        self.get_window_mut(handle)
            .and_then(|window| window.input_receiver())
            .and_then(|receiver| receiver.try_receive().ok())
    }

    /// Try to send an input event to the specified window.
    pub async fn try_send_input(&mut self, handle: WindowHandle, event: HumanInputEvent) -> Result<(), ()> {
        if let Some(window) = self.get_window_mut(handle) {
            if let Some(sender) = window.input_sender().await {
                sender.try_send(event).map_err(|_| ())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    // No dirty-region helpers here; Canvas owns dirty tracking.

    // --- Internal helpers ---
    fn index_of(&self, handle: WindowHandle) -> Option<usize> {
        self.windows.iter().position(|w| w.handle() == handle)
    }

    fn get_window(&self, handle: WindowHandle) -> Option<&Window> {
        self.windows.iter().find(|w| w.handle() == handle)
    }

    pub fn get_window_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }
}


