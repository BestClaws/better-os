use alloc::boxed::Box;
use core::sync::atomic::{AtomicU32, Ordering};
use defmt::info;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::semaphore::Semaphore;
use embassy_time::{Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::{
    SubmitFrame, BUFFER_USED, FB_SEMAPHORE, FRAMEBUFFERS, FRAME_CHANNEL,
};
use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};

const MAX_APPS: usize = 4;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const FRAME_SIZE: usize = WIDTH * HEIGHT / 8;

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
                    if let Some(old_id) = last_frame_id[app_id] {
                        BUFFER_USED.fetch_and(!(1 << old_id), Ordering::SeqCst);
                        FB_SEMAPHORE.release(1);
                    }
                    last_frame_id[app_id] = Some(id);

                    if app_id == current_app {
                        let buf = unsafe { &FRAMEBUFFERS[id] };
                        let mut disp = display_hal.lock().await;
                        disp.draw(buf).await;
                    }
                } else {
                    BUFFER_USED.fetch_and(!(1 << id), Ordering::SeqCst);
                    FB_SEMAPHORE.release(1);
                }
            }

            Either::Second(event) => {
                let old_app = current_app;
                match event {
                    HumanInputEvent::NavUp => {
                        current_app = (current_app + 1) % MAX_APPS;
                        info!("Switched to app {}", current_app);
                    }
                    HumanInputEvent::NavDown => {
                        current_app = (current_app + MAX_APPS - 1) % MAX_APPS;
                        info!("Switched to app {}", current_app);
                    }
                    _ => {}
                }

                if let (Some(from_id), Some(to_id)) = (last_frame_id[old_app], last_frame_id[current_app]) {
                    let from_buf = unsafe { &FRAMEBUFFERS[from_id] };
                    let to_buf = unsafe { &FRAMEBUFFERS[to_id] };

                    let mut disp = display_hal.lock().await;
                    for step in 0..=WIDTH {
                        let mut frame = [0u8; FRAME_SIZE];

                        for y in 0..HEIGHT {
                            for x in 0..WIDTH {
                                let byte_index = x + (y / 8) * WIDTH;
                                let bit_index = y % 8;

                                let from_x = x as isize - (step as isize);
                                let to_x = x as isize - (step as isize) + WIDTH as isize;

                                let mut val = 0;
                                if (0..WIDTH as isize).contains(&from_x) {
                                    let from_index = from_x as usize + (y / 8) * WIDTH;
                                    if from_buf[from_index] & (1 << bit_index) != 0 {
                                        val |= 1;
                                    }
                                }

                                if (0..WIDTH as isize).contains(&to_x) {
                                    let to_index = to_x as usize + (y / 8) * WIDTH;
                                    if to_buf[to_index] & (1 << bit_index) != 0 {
                                        val |= 1;
                                    }
                                }

                                if val != 0 {
                                    frame[byte_index] |= 1 << bit_index;
                                }
                            }
                        }

                        disp.draw(&frame).await;
                        Timer::after_millis(8).await;
                    }
                }
            }
        }

        if let Some(id) = last_frame_id[current_app] {
            let buf = unsafe { &FRAMEBUFFERS[id] };
            let mut disp = display_hal.lock().await;
            disp.draw(buf).await;
        }

        Timer::after_millis(33).await;
    }
}
