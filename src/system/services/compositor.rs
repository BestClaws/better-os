use alloc::boxed::Box;
use core::sync::atomic::{AtomicU32, Ordering};
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Instant, Timer};
use embassy_futures::select::{select, Either};
use embassy_sync::semaphore::Semaphore;
use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::{SubmitFrame, BUFFER_USED, FB_SEMAPHORE, FRAME_CHANNEL, FRAMEBUFFERS};
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};

const MAX_APPS: usize = 4;

#[embassy_executor::task]
pub async fn compositor_service(
    display_hal: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!(
        "[{}s] compositor service started",
        Instant::now().as_millis() as f32 / 1000f32
    );

    {
        let mut disp = display_hal.lock().await;
        disp.init().await;
    }

    let mut current_app = 0;
    let mut last_frame_id: [Option<usize>; MAX_APPS] = [None; MAX_APPS];

    let frame_rx = FRAME_CHANNEL.receiver();
    let input_rx = HUMAN_INPUT_CH.receiver();

    loop {
        match select(frame_rx.receive(), input_rx.receive()).await {
            Either::First(SubmitFrame { id, app_id }) => {
                if app_id < MAX_APPS {
                    // Store the latest frame from this app
                    if let Some(old_id) = last_frame_id[app_id] {
                        BUFFER_USED.fetch_and(!(1 << old_id), Ordering::SeqCst);
                        FB_SEMAPHORE.release(1);
                    }

                    last_frame_id[app_id] = Some(id);

                    if app_id == current_app {
                        let buf = unsafe { &FRAMEBUFFERS[id] };
                        let mut disp = display_hal.lock().await;
                        disp.draw(buf).await; // still no draw_buf
                    }
                } else {
                    // Invalid app_id — release immediately
                    BUFFER_USED.fetch_and(!(1 << id), Ordering::SeqCst);
                    FB_SEMAPHORE.release(1);
                }
            }

            Either::Second(event) => match event {
                HumanInputEvent::NavUp => {
                    current_app = (current_app + 1) % MAX_APPS;
                    info!("Switched to app {}", current_app);
                }
                HumanInputEvent::NavDown => {
                    current_app = (current_app + MAX_APPS - 1) % MAX_APPS;
                    info!("Switched to app {}", current_app);
                }
                _ => {}
            },
        }

        // Draw current app’s last frame
        if let Some(id) = last_frame_id[current_app] {

            let buf = unsafe { &FRAMEBUFFERS[id] };
            let mut disp = display_hal.lock().await;
            disp.draw(buf).await;

        }

        Timer::after_millis(33).await;
    }
}
