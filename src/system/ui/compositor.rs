use core::cmp::PartialEq;
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
pub struct UICompositor {
    windows: heapless::Vec<Window, 8>,
    current_index: usize,
    view_mode: ViewMode,
    composited_id: Option<usize>,
    next_id: usize,
}



impl UICompositor {
    pub fn new() -> Self {
        Self {
            windows: heapless::Vec::new(),
            current_index: 0,
            view_mode: ViewMode::Single,
            composited_id: None,
            next_id: 0,
        }
    }

    pub async fn alloc_window_with_canvas(
        &mut self,
        width: usize,
        height: usize,
        id: usize,
    ) -> Option<(WindowHandle, Canvas)> {
        let fb = allocate_buffer().await?;
        let window = Window::new(fb, width, height, id);
        let handle = window.handle();

        // Push the window first
        self.windows.push(window).ok()?;

        // Now get mutable ref to it and extract canvas
        let window = self.windows.iter_mut().find(|w| w.handle() == handle)?;
        let canvas = window.canvas(); // dynamically generated

        Some((handle, canvas))
    }

    /// Allocates and registers a new window. Returns a handle to it.
    pub async fn alloc_window(&mut self, width: usize, height: usize) -> Option<WindowHandle> {
        let fb = allocate_buffer().await?;
        let id = self.next_id;
        self.next_id += 1;

        let window = Window::new(fb, width, height, id);
        let handle = window.handle();

        self.windows.push(window).ok()?;
        Some(handle)
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

    pub async fn composite(&mut self) -> Option<&'static [u8]> {
        let mut fb = allocate_buffer().await?;
        let id = fb.id();
        let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
        composed.clear();

        match self.view_mode {
            ViewMode::Single => {
                if let Some(window) = self.windows.get_mut(self.current_index) {
                    let mut canvas = window.canvas();
                    composed.draw_from(&canvas, 0, 0);
                }
            }
            ViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.windows.len();

                if let Some(w1) = self.windows.get_mut(i1) {
                    let mut canvas1 = w1.canvas();
                    composed.draw_from(&canvas1, 0, 0);
                }

                if let Some(w2) = self.windows.get_mut(i2) {
                    let mut canvas2 = w2.canvas();
                    composed.draw_from(&canvas2, 0, 32);
                }
            }
        }

        self.composited_id = Some(id);
        Some(get_buffer_slice(id))
    }

    pub fn release_last(&mut self) {
        if let Some(id) = self.composited_id.take() {
            release_buffer(id);
        }
    }
}
