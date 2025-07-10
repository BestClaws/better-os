use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{SlideDir, UICompositor, ViewMode};

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
) {
    {
        let mut comp = compositor.lock().await;
        comp.attach_display(display);
    }

    loop {
        let event = HUMAN_INPUT_CH.receive().await;

        let mut comp = compositor.lock().await;

        let mut slide_direction = None;
        let mut trigger_redraw = false;

        match event {
            HumanInputEvent::NavUp => {
                comp.prev_window();
                slide_direction = Some(SlideDir::Right);
            }
            HumanInputEvent::NavDown => {
                comp.next_window();
                slide_direction = Some(SlideDir::Left);
            }
            HumanInputEvent::OkPressed => {
                comp.toggle_view();
                trigger_redraw = true;
            }
            _ => {
                // Forward event to current window's input channel
                let current = comp.current_handle();
                if let Some(window) = comp.window_for_handle_mut(current) {
                    let _ = window.input_sender().try_send(event); // best-effort
                }
            }
        }

        if let Some(dir) = slide_direction {
            comp.animate_slide(dir).await;
        }

        if trigger_redraw {
            let current = comp.current_handle();
            comp.request_redraw(current);
        }

        comp.step().await;
    }
}
