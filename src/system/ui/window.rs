use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::{FrameBufferHandle};

/// Strongly-typed handle to a window.
#[derive(Clone, Copy, PartialEq)]
pub struct WindowHandle {
    id: usize,
}

/// Represents a single UI window, backed by a framebuffer.
pub struct Window {
    fb: FrameBufferHandle,
    width: u32,
    height: u32,
    id: usize, // App ID or Window ID — anything you use to uniquely identify this window
}

impl Window {
    /// Create a new Window.
    pub fn new(fb: FrameBufferHandle, width: usize, height: usize, id: usize) -> Self {
        Self {
            fb,
            width: width as u32,
            height: height as u32,
            id,
        }
    }

    /// Returns a fresh Canvas that draws on this window's framebuffer.
    pub fn canvas(&mut self) -> Canvas {
        Canvas::new(self.fb.buffer_mut(), self.width, self.height)
    }

    /// Return this window’s handle.
    pub fn handle(&self) -> WindowHandle {
        WindowHandle { id: self.id }
    }

    /// Get ID of the framebuffer (for compositing).
    pub fn framebuffer_id(&self) -> usize {
        self.fb.id()
    }

    /// Return this window’s raw width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Return this window’s raw height.
    pub fn height(&self) -> u32 {
        self.height
    }
}
