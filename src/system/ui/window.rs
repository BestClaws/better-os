use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::FrameBufferHandle;

/// A window managed by the compositor.
pub struct Window<'a> {
    width: u32,
    height: u32,
    canvas: Canvas<'a>,
    fb_handle: FrameBufferHandle,
}

impl<'a> Window<'a> {
    pub fn new(canvas: Canvas<'a>, fb_handle: FrameBufferHandle) -> Self {

        let width = canvas.width();
        let height = canvas.height();
        Self {
            canvas,
            width,
            height,
            fb_handle,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.canvas.resize(width, height);
    }

    /// Returns a handle that can be passed to app code.
    pub fn handle(&'a mut self) -> WindowHandle<'a> {
        WindowHandle {
            width: self.width,
            height: self.height,
            canvas: &mut self.canvas,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn canvas(&mut self) -> &mut Canvas<'a> {
        &mut self.canvas
    }
}

/// A safe handle passed to apps for rendering.
pub struct WindowHandle<'a> {
    width: u32,
    height: u32,
    canvas: &'a mut Canvas<'a>,
}

impl<'a> WindowHandle<'a> {
    pub fn canvas(&mut self) -> &mut Canvas<'a> {
        self.canvas
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
