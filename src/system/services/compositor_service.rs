use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Timer, Duration};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{UICompositor, ViewMode};
use defmt::{info, warn};

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor<'static>>,
) {
    loop {
        let event = HUMAN_INPUT_CH.receive().await;

        let mut comp = compositor.lock().await;
        let mut changed = false;

        match event {
            HumanInputEvent::NavUp => {
                comp.prev_window();
                changed = true;
            }
            HumanInputEvent::NavDown => {
                comp.next_window();
                changed = true;
            }
            HumanInputEvent::OkPressed => {
                comp.toggle_view();
                changed = true;
            }
            _ => {}
        }

        if changed {
            comp.release_last();

            // Animate left-to-right slide
            if let Some(from_fb) = comp.composite().await {
                for offset in (0..=128).step_by(8) {
                    if let Some(next_fb) = comp.composite_with_offset(offset).await {
                        let mut disp = display.lock().await;
                        disp.draw(next_fb).await;
                        Timer::after(Duration::from_millis(10)).await;
                    }
                }

                comp.release_last();
            }
        }
    }
}
