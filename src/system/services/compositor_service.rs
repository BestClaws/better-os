use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Timer, Duration};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{SlideDir, UICompositor, ViewMode};
use defmt::{info, warn};

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
) {
    loop {
        let event = HUMAN_INPUT_CH.receive().await;

        let mut comp = compositor.lock().await;
        let mut changed = false;
        let mut slide_direction: Option<SlideDir> = None;

        match event {
            HumanInputEvent::NavUp => {
                comp.prev_window();
                changed = true;
                slide_direction = Some(SlideDir::Right);
            }
            HumanInputEvent::NavDown => {
                comp.next_window();
                changed = true;
                slide_direction = Some(SlideDir::Left);
            }
            HumanInputEvent::OkPressed => {
                comp.toggle_view();
                changed = true;
            }
            _ => {}
        }

        if changed {
            comp.release_last();

            match comp.view_mode() {
                ViewMode::Single => {
                    if let Some(dir) = slide_direction {
                        // Animate slide from previous to current
                        if let Some((from_idx, to_idx)) = comp.last_transition_handles(dir) {
                            for offset in (0..=128).step_by(8) {
                                let offset = match dir {
                                    SlideDir::Left => offset,
                                    SlideDir::Right => 128 - offset,
                                };

                                if let Some(frame) = comp.composite_slide(from_idx, to_idx, offset).await {
                                    let mut disp = display.lock().await;
                                    disp.draw(frame).await;
                                    Timer::after(Duration::from_millis(10)).await;
                                    comp.release_last();
                                }
                            }
                        }
                    }
                }
                ViewMode::Split => {
                    if let Some(frame) = comp.composite().await {
                        let mut disp = display.lock().await;
                        disp.draw(frame).await;
                        comp.release_last();
                    }
                }
            }
        }
    }
}

