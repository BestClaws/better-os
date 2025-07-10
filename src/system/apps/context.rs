use crate::system::ui::window::WindowHandle;

pub struct AppContext<'a> {
    pub app_id: usize,
    pub window: WindowHandle<'a>,
}

impl<'a> AppContext<'a> {
    pub fn new(app_id: usize, window: WindowHandle<'a>) -> Self {
        Self { app_id, window }
    }

    pub fn canvas(&mut self) -> &mut crate::system::ui::canvas::Canvas<'a> {
        self.window.canvas()
    }
}
