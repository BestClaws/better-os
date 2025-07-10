use crate::system::ui::canvas::Canvas;

/// Represents a drawable region on the display.
/// A `Window` owns a `Canvas` and includes metadata like size and position.
/// The compositor can resize this window dynamically.
pub struct Window<'a> {
    app_id: usize,
    width: u32,
    height: u32,
    canvas: Canvas<'a>,
}

impl<'a> Window<'a> {
    /// Create a new window with a given canvas.
    pub fn new(app_id: usize, canvas: Canvas<'a>, width: u32, height: u32) -> Self {
        Self {
            app_id,
            canvas,
            width,
            height,
        }
    }

    /// Get the window's width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the window's height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// App ID that owns this window.
    pub fn app_id(&self) -> usize {
        self.app_id
    }

    /// Mutable access to the drawable canvas.
    pub fn canvas(&mut self) -> &mut Canvas<'a> {
        &mut self.canvas
    }

    /// Notify window of a resize event (used by compositor).
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.canvas.resize(width, height);
    }
}
