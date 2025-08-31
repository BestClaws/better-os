use alloc::boxed::Box;
use defmt::export::display;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, Instant};

use crate::system::hal::display::AsyncDisplay;
use crate::system::kernel::config::resources::FRAME_BUFFER_WIDTH;
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{SlideDir, UICompositor};

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
) {

        display.lock().await.init().await;

    display.lock().await.set_brightness(0xff).await;


    // let mut color: u32 = 1;
    // 
    // loop {
    //     color += 100;
    //     color = color % 255;
    // 
    //     display.lock().await.paint_screen(color as u8).await;
    //     Timer::after_millis(1000).await;
    // 
    //     info!("next color");
    // 
    // }





    {
        let mut comp = compositor.lock().await;
        comp.attach_display(display);
    }

    loop {
        let idle_start = Instant::now();
        let mut last_slide: Option<SlideDir> = None;
        let mut toggle_view = false;

        let mut event = None;

        // Drain all available input events
        while let Ok(e) = HUMAN_INPUT_CH.try_receive() {
            event = Some(e);
        }

        if let Some(event) = event {
            match event {
                // HumanInputEvent::Touch(x, y) => {
                //
                //     if (x < (FRAME_BUFFER_WIDTH / 2) as i32) {
                //         last_slide = Some(SlideDir::Right);
                //     } else {
                //         last_slide = Some(SlideDir::Left);
                //
                //     }
                //
                // }
                HumanInputEvent::OkPressed => {
                    toggle_view = true;
                }
                other => {
                    // Forward input to current app
                    let mut comp = compositor.lock().await;
                    let current = comp.current_handle().unwrap();
                    if let Some(window) = comp.get_window_mut(current) {
                        let _ = window.input_sender().await.unwrap().try_send(other);
                    }
                }
            }

        }





        // Apply slide or view toggle logic
        if let Some(dir) = last_slide {
            let mut comp = compositor.lock().await;
            comp.animate_slide(dir).await;
            comp.step().await;
        } else if toggle_view {
            let mut comp = compositor.lock().await;
            comp.toggle_view();
            let current = comp.current_handle().unwrap();
            comp.request_redraw(current);
            comp.step().await;
        } else {

            // No user input: step once every ~100ms for idle refresh
            Timer::after(Duration::from_millis(100)).await;
            let Some(current) = compositor.lock().await.current_handle() else {
                Timer::after_nanos(0).await;
                continue;
            };
            compositor.lock().await.request_redraw(current); // passive draw (for e.g. clock, sensor UI)
            compositor.lock().await.step().await;


        }


        // Sleep remaining time if loop was too fast (ensure ~10Hz)
        let elapsed = Instant::now() - idle_start;
        if elapsed < Duration::from_millis(30) {
            Timer::after(Duration::from_millis(100) - elapsed).await;
        }
    }
}
