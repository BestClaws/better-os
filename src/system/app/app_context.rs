#![allow(unused)]


use crate::system::ui::window::WindowHandle;
use crate::system::ui::canvas::Canvas;
use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::compositor::UICompositor;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

pub struct AppContext<'a> {
    pub handle: WindowHandle,
    pub canvas: Canvas<'a>,
    pub app_id: usize,
    pub app_name: &'static str,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
}

impl<'a> AppContext<'a> {
    pub fn new(
        handle: WindowHandle,
        canvas: Canvas<'a>,
        app_id: usize,
        app_name: &'static str,
        compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    ) -> Self {
        Self {
            handle,
            canvas,
            app_id,
            app_name,
            compositor,
        }
    }

    pub fn width(&self) -> u32 {
        self.canvas.width()
    }

    pub fn height(&self) -> u32 {
        self.canvas.height()
    }

    pub async fn is_focused(&self) -> bool {
        let comp = self.compositor.lock().await;
        comp.is_focused(self.handle)
    }

    pub async fn poll_input(&self) -> Option<HumanInputEvent> {
        let mut comp = self.compositor.lock().await;
        comp.poll_input(self.handle)
    }

    pub async fn request_redraw(&self) {
        let mut comp = self.compositor.lock().await;
        comp.request_redraw(self.handle);
    }
}
