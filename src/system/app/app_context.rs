
use crate::system::input::types::HighLevelEvent;
use crate::system::ui::canvas::DrawingSurface as Canvas;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window::WindowHandle;
use crate::system::ui::window_manager::WindowManager;
use crate::libs::ui::imui::{ImInput};
use crate::system::input::types::{MotionEvent, TouchAction};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::libs::gfx::two_d::Canvas2D;

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
        let comp = self.compositor.lock().await;
        comp.is_window_focused(self.handle)
    }

    pub async fn poll_input(&self) -> Option<HighLevelEvent> {
        let mut wm = self.window_manager.lock().await;
        wm.poll_window_input(self.handle)
    }

    pub async fn request_redraw(&self) {
        let mut comp = self.compositor.lock().await;
        comp.request_redraw(self.handle);
    }

    pub async fn draw(&self, f: impl FnOnce(&mut Canvas) + Send) {
        let mut wm = self.window_manager.lock().await;
        let _ = wm.with_canvas(self.handle, f);
    }

    // Note: To use RGBA drawing, construct Canvas2D within the draw() closure:
    // context.draw(|canvas| { let mut c2d = Canvas2D::new(canvas); /* use fluent .draw(&mut c2d) */ });

    /// Gather pending input events and provide an `ImInput` snapshot, then draw.
    pub async fn draw_immediate(&self, f: impl FnOnce(&mut Canvas, ImInput) + Send) {
        // Collect input events into a single snapshot for this frame
        let mut wm = self.window_manager.lock().await;
        let mut input = ImInput::default();
        loop {
            if let Some(ev) = wm.poll_window_input(self.handle) {
                match ev {
                    HighLevelEvent::Motion(MotionEvent { action, pointers, .. }) => {
                        if let Some(p) = pointers[0] {
                            input.pointer_pos = Some(crate::libs::gfx::two_d::Point::new(p.x, p.y));
                            match action {
                                TouchAction::Down => { input.pointer_down = true; }
                                TouchAction::Up => { input.pointer_released = true; input.pointer_down = false; }
                                TouchAction::Move => { /* pos updated */ }
                            }
                        }
                    }
                    _ => {}
                }
            } else { break; }
        }

        let _ = wm.with_canvas(self.handle, |canvas| f(canvas, input));
    }
}
