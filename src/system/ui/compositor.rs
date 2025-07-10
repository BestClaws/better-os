use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::{allocate_buffer, get_buffer_slice, release_buffer};
use crate::system::ui::window::{Window, WindowHandle};

/// Whether we're showing a single window or split view
#[derive(Clone, Copy, Debug)]
pub enum ViewMode {
    Single,
    Split,
}

/// Compositor struct managing windows and rendering logic.
pub struct UICompositor<'a> {
    windows: heapless::Vec<Window<'a>, 8>,
    current_index: usize,
    view_mode: ViewMode,
    composited_id: Option<usize>,
}

impl<'a> UICompositor<'a> {
    pub fn new() -> Self {
        Self {
            windows: heapless::Vec::new(),
            current_index: 0,
            view_mode: ViewMode::Single,
            composited_id: None,
        }
    }

    /// Allocates and registers a new window. Returns a handle to it.
    pub async fn alloc_window(&'a mut self, width: usize, height: usize) -> Option<WindowHandle<'a>> {
        let mut fb = allocate_buffer().await?;
        let canvas = Canvas::new(fb.buffer_mut(), width as u32, height as u32);
        let window = Window::new(canvas, fb);

        let idx = self.windows.len();
        self.windows.push(window).ok()?;
        let window = self.windows.get_mut(idx)?;

        Some(window.handle())
    }


    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Single => ViewMode::Split,
            ViewMode::Split => ViewMode::Single,
        };
    }

    pub fn next_window(&mut self) {
        if !self.windows.is_empty() {
            self.current_index = (self.current_index + 1) % self.windows.len();
        }
    }

    pub fn prev_window(&mut self) {
        if !self.windows.is_empty() {
            self.current_index = (self.current_index + self.windows.len() - 1) % self.windows.len();
        }
    }

    /// Composites the visible window(s) into a framebuffer and returns its slice.
    pub async fn composite(&mut self) -> Option<&'static [u8]> {
        let mut fb = allocate_buffer().await?;
        let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
        composed.clear();

        match self.view_mode {
            ViewMode::Single => {
                if let Some(window) = self.windows.get_mut(self.current_index) {
                    composed.draw_from(window.canvas(), 0, 0);
                }
            }
            ViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.windows.len();

                if let Some(w1) = self.windows.get_mut(i1) {
                    composed.draw_from(w1.canvas(), 0, 0);
                }

                if let Some(w2) = self.windows.get_mut(i2) {
                    composed.draw_from(w2.canvas(), 0, 32);
                }
            }
        }

        let id = fb.id();
        self.composited_id = Some(id);
        Some(get_buffer_slice(id))
    }

    pub fn release_last(&mut self) {
        if let Some(id) = self.composited_id.take() {
            release_buffer(id);
        }
    }
}
