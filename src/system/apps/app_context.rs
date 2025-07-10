use crate::system::ui::canvas::Canvas;
use crate::system::ui::window::WindowHandle;

/// A context passed to each UI app when it starts.
/// Contains its drawable canvas and metadata.
pub struct AppContext<'a> {
    pub handle: WindowHandle,
    pub canvas: Canvas<'a>,
    pub app_id: usize,
    pub app_name: &'static str,
}

impl<'a> AppContext<'a> {
    pub fn new(handle: WindowHandle, canvas: Canvas<'a>, app_id: usize, app_name: &'static str) -> Self {
        Self {
            handle,
            canvas,
            app_id,
            app_name,
        }
    }

    pub fn width(&self) -> u32 {
        self.canvas.width()
    }

    pub fn height(&self) -> u32 {
        self.canvas.height()
    }
}
