use crate::system::hal::display::DisplayResolution;
use crate::system::input::types::HighLevelEvent;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::system::ui::windowing::{WindowHandle, WindowManager};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
// migrated away from two_d Canvas2D; apps use gfx::Rasterizer directly

pub struct AppContext {
    pub handle: WindowHandle,
    pub app_id: usize,
    pub app_name: &'static str,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
}

impl AppContext {
    pub fn new(
        handle: WindowHandle,
        app_id: usize,
        app_name: &'static str,
        compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
        window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    ) -> Self {
        Self {
            handle,
            app_id,
            app_name,
            compositor,
            window_manager,
        }
    }

    pub async fn is_focused(&self) -> bool {
        let mut comp = self.compositor.lock().await;
        if comp.is_window_focused(self.handle) {
            true
        } else {
            comp.take_warmup_request(self.handle)
        }
    }

    pub async fn poll_input(&self) -> Option<HighLevelEvent> {
        let mut wm = self.window_manager.lock().await;
        wm.poll_window_input(self.handle)
    }

    pub async fn request_redraw(&self) {
        let mut comp = self.compositor.lock().await;
        comp.request_redraw(self.handle);
    }

    pub async fn draw(&self, f: impl FnOnce(&mut DrawingSurface) + Send) {
        let mut wm = self.window_manager.lock().await;
        let _ = wm.with_surface(self.handle, f);
    }

    pub async fn resize_window(&self, width: u32, height: u32) -> Result<(), ()> {
        let mut wm = self.window_manager.lock().await;
        wm.resize_window(self.handle, width, height)
    }

    pub async fn default_window_resolution(&self) -> Option<DisplayResolution> {
        let wm = self.window_manager.lock().await;
        wm.default_resolution()
    }
}
