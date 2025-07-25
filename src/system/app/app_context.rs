use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::{Canvas, PixelColorExt};
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window::WindowHandle;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_graphics_core::pixelcolor::PixelColor;

pub struct AppContext {
    pub handle: WindowHandle,
    pub app_id: usize,
    pub app_name: &'static str,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
}

impl AppContext {
    pub fn new(
        handle: WindowHandle,
        app_id: usize,
        app_name: &'static str,
        compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    ) -> Self {
        Self {
            handle,
            app_id,
            app_name,
            compositor,
        }
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

    pub async fn draw<C: PixelColor + PixelColorExt>(&self, f: impl FnOnce(&mut Canvas<C>) + Send) {
        let mut comp = self.compositor.lock().await;
        if let Some(window) = comp.get_window_mut(self.handle) {
            let mut canvas = window.canvas().await;
            f(&mut canvas);
        }
    }
}
