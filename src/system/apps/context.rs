use crate::system::ui::canvas::Canvas;
use crate::system::ui::window::WindowHandle;

/// Basic app metadata (can be extended).
pub struct AppMetadata {
    pub name: &'static str,
    pub size: (u32, u32),
}

/// The runtime context given to every app.
pub struct AppContext<'a> {
    pub app_id: usize,
    pub window: WindowHandle,
    pub canvas: Canvas<'a>,
    pub metadata: AppMetadata,
}
