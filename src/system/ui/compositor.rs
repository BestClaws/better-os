use crate::system::ui::framebuffer::{allocate_buffer, release_buffer, get_buffer_slice, FrameBufferHandle};
use crate::system::ui::window::Window;
use crate::system::ui::canvas::Canvas;

/// ID to represent an app
pub type AppId = usize;

/// Whether we're showing a single app or split view
#[derive(Clone, Copy, Debug)]
pub enum AppViewMode {
    Single,
    Split,
}

/// Internal state for one app
struct AppSlot {
    pub app_id: AppId,
    pub window: Option<Window<'static>>,
}

/// The main UI compositor struct.
/// It owns windows, manages transitions and tracks app view state.
pub struct UICompositor {
    apps: heapless::Vec<AppSlot, 8>,
    current_index: usize,
    view_mode: AppViewMode,
    composited_id: Option<usize>, // Last composited buffer
}

impl UICompositor {
    pub fn new() -> Self {
        Self {
            apps: heapless::Vec::new(),
            current_index: 0,
            view_mode: AppViewMode::Single,
            composited_id: None,
        }
    }

    /// Register a new app with a window.
    pub fn register_app(&mut self, app_id: AppId) {
        self.apps.push(AppSlot { app_id, window: None }).ok();
    }

    /// Submit a frame for an app.
    pub fn submit_frame(&mut self, app_id: AppId, canvas: Canvas<'static>, buffer_id: usize) {
        if let Some(slot) = self.apps.iter_mut().find(|a| a.app_id == app_id) {
            slot.window = Some(Window::new(app_id, canvas, 128, 64));
            // Note: Resize support can go here in future
        }
    }

    /// Get current app ID (used by navigation logic)
    pub fn current_app_id(&self) -> Option<AppId> {
        self.apps.get(self.current_index).map(|a| a.app_id)
    }

    /// Move to next app
    pub fn next_app(&mut self) {
        if !self.apps.is_empty() {
            self.current_index = (self.current_index + 1) % self.apps.len();
        }
    }

    /// Move to previous app
    pub fn prev_app(&mut self) {
        if !self.apps.is_empty() {
            self.current_index = (self.current_index + self.apps.len() - 1) % self.apps.len();
        }
    }

    /// Toggle between single and split view
    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            AppViewMode::Single => AppViewMode::Split,
            AppViewMode::Split => AppViewMode::Single,
        };
    }

    /// Compose the current frame into a framebuffer and return its ID.
    pub fn composite(&mut self) -> Option<&'static [u8]> {
        let fb = allocate_buffer()?;
        let canvas = Canvas::new(fb.buffer_mut(), 128, 64);
        let mut composed = canvas;

        composed.clear();

        match self.view_mode {
            AppViewMode::Single => {
                if let Some(slot) = self.apps.get(self.current_index) {
                    if let Some(window) = &slot.window {
                        composed.draw(window.canvas()).ok();
                    }
                }
            }
            AppViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.apps.len();

                if let Some(w1) = self.apps.get(i1).and_then(|a| a.window.as_ref()) {
                    let mut top = w1.canvas().clipped(0, 0, 128, 32);
                    composed.draw(&top).ok();
                }
                if let Some(w2) = self.apps.get(i2).and_then(|a| a.window.as_ref()) {
                    let mut bot = w2.canvas().clipped(0, 32, 128, 32);
                    composed.draw(&bot).ok();
                }
            }
        }

        self.composited_id = Some(fb.id());
        Some(fb.buffer())
    }

    /// Release the last composited buffer after display
    pub fn release_last(&mut self) {
        if let Some(id) = self.composited_id.take() {
            release_buffer(id);
        }
    }
}
