use alloc::vec::Vec;
use defmt::{debug, warn};

use super::window::{Window, WindowHandle};
use crate::system::hal::display::{DisplayResolution, PixelFormat};
use crate::system::input::types::HighLevelEvent;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::resources::input_channels::INPUT_CHANNEL_POOL;
use crate::system::ui::drawing_surface::DrawingSurface;

/// Maximum supported windows in the system. Matches the compositor limit.
const MAX_WINDOWS: usize = 8;

/// Manages windows and their scarce resources (framebuffers and input channels).
///
/// Responsibilities:
/// - Create and destroy windows
/// - Allocate/reclaim resources for a chosen active set (up to 3 windows)
/// - Provide access to canvases and input channels
/// - Track one-shot initial paint requests for pre-active (next/previous) windows
pub struct WindowManager {
    windows: Vec<Window>,
    display_format: PixelFormat,
    default_resolution: Option<DisplayResolution>,
}

impl WindowManager {
    /// Create a new empty window manager with a fallback PixelFormat.
    pub fn new(default_format: PixelFormat) -> Self {
        Self {
            windows: Vec::with_capacity(MAX_WINDOWS),
            display_format: default_format,
            default_resolution: None,
        }
    }

    pub fn has_display_config(&self) -> bool {
        self.default_resolution.is_some()
    }

    pub fn default_resolution(&self) -> Option<DisplayResolution> {
        self.default_resolution
    }

    pub fn default_dimensions(&self) -> Option<(u32, u32)> {
        self.default_resolution
            .map(|resolution| (resolution.logical.width, resolution.logical.height))
    }

    pub fn display_format(&self) -> PixelFormat {
        self.display_format
    }

    /// Update negotiated display configuration and propagate to existing windows.
    pub fn update_display_config(&mut self, resolution: DisplayResolution, format: PixelFormat) {
        let previous_default = self.default_resolution;
        self.display_format = format;
        self.default_resolution = Some(resolution);

        let previous_dims = previous_default.map(|res| (res.logical.width, res.logical.height));
        let negotiated_dims = (resolution.logical.width, resolution.logical.height);

        for window in self.windows.iter_mut() {
            let current_dims = (window.width(), window.height());
            let adopt_negotiated = match previous_dims {
                None => true,
                Some(prev) => current_dims == prev,
            };
            let (target_w, target_h) = if adopt_negotiated {
                negotiated_dims
            } else {
                current_dims
            };
            window.update_surface(target_w, target_h, self.display_format);
        }
    }

    /// Create a new window and add it to the manager using the negotiated dimensions.
    pub async fn create_window(&mut self, id: usize) -> Option<WindowHandle> {
        let default = match self.default_resolution() {
            Some(res) => res,
            None => {
                warn!("WindowManager lacks display metrics; refusing to create window");
                return None;
            }
        };
        self.create_window_with_size(id, default.logical.width, default.logical.height)
            .await
    }

    /// Create a new window using caller-provided logical dimensions.
    pub async fn create_window_with_size(
        &mut self,
        id: usize,
        mut width: u32,
        mut height: u32,
    ) -> Option<WindowHandle> {
        if width == 0 || height == 0 {
            warn!(
                "Refusing to create window id={} with zero dimension ({}x{})",
                id, width, height
            );
            return None;
        }

        if self.windows.len() >= MAX_WINDOWS {
            warn!("WindowManager capacity reached; cannot create more windows");
            return None;
        }

        if let Some(default) = self.default_resolution {
            width = width.min(default.logical.width);
            height = height.min(default.logical.height);
        }

        let window = Window::new(width, height, id, self.display_format).await;
        let handle = window.handle();
        self.windows.push(window);
        debug!("Window created: id={}, size={}x{}", id, width, height);
        Some(handle)
    }

    /// Destroy a window and reclaim any resources held by it.
    pub fn destroy_window(&mut self, handle: WindowHandle) -> bool {
        if let Some(idx) = self.index_of(handle) {
            let mut window = self.windows.swap_remove(idx);
            if window.framebuffer_id().is_some() {
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
                debug!(
                    "Releasing resources for inactive window {:?}",
                    window.handle()
                );
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
                        Some(fb) => match INPUT_CHANNEL_POOL.allocate().await {
                            Some(ic) => {
                                let (w, h) = (window.width(), window.height());
                                let surface_slot = window.surface();
                                match surface_slot {
                                    slot @ None => {
                                        *slot = Some(DrawingSurface::new_unattached(
                                            w,
                                            h,
                                            self.display_format,
                                        ));
                                    }
                                    Some(surface) => {
                                        surface.reconfigure(w, h, self.display_format);
                                    }
                                }
                                window.set_resources(fb, ic).await;
                            }
                            None => {
                                warn!("Input channel allocation failed for window {:?}", handle);
                                FRAMEBUFFER_POOL.release(&fb);
                            }
                        },
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

    /// Provide mutable access to a window's drawing surface for rendering.
    pub fn with_surface<R>(
        &mut self,
        handle: WindowHandle,
        f: impl FnOnce(&mut DrawingSurface) -> R,
    ) -> Option<R> {
        let window = self.get_window_mut(handle)?;
        let surface_opt = window.surface();
        surface_opt.as_mut().map(f)
    }

    /// Get a single pending input event for the window, if any.
    pub fn poll_window_input(&mut self, handle: WindowHandle) -> Option<HighLevelEvent> {
        self.get_window_mut(handle)
            .and_then(|window| window.input_receiver())
            .and_then(|receiver| receiver.try_receive().ok())
    }

    /// Try to send an input event to the specified window.
    pub async fn try_send_input(
        &mut self,
        handle: WindowHandle,
        event: HighLevelEvent,
    ) -> Result<(), ()> {
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

    /// Resize a window to new logical dimensions while preserving pixel format.
    pub fn resize_window(
        &mut self,
        handle: WindowHandle,
        mut width: u32,
        mut height: u32,
    ) -> Result<(), ()> {
        if width == 0 || height == 0 {
            return Err(());
        }

        if let Some(default) = self.default_resolution {
            width = width.min(default.logical.width);
            height = height.min(default.logical.height);
        }

        let format = self.display_format;

        if let Some(window) = self.get_window_mut(handle) {
            window.update_surface(width, height, format);
            Ok(())
        } else {
            Err(())
        }
    }

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
