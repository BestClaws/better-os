use alloc::boxed::Box;
use defmt::export::display;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, Instant};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{SlideDir, UICompositor};
use crate::system::vendor::boby::drivers::ili9341::driver::Orientation;

#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    _compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
) {

    {
        display.lock().await.init().await;
        
    }


    loop {
        info!("Display clear");
        {
            display.lock().await.clear(2016).await;
            display.lock().await.set_orientation(Orientation::Landscape).await;
        }
        info!("Display clear done");
        Timer::after(Duration::from_millis(3000)).await;
    }


    // {
    //     display.lock().await.init().await;
    // }
    //


    // {
    //     let mut comp = compositor.lock().await;
    //     comp.attach_display(display);
    // }

    // loop {
    //     let idle_start = Instant::now();
    //     let mut last_slide: Option<SlideDir> = None;
    //     let mut toggle_view = false;
    //
    //     // Drain all available input events
    //     while let Ok(event) = HUMAN_INPUT_CH.try_receive() {
    //         match event {
    //             HumanInputEvent::NavUp => {
    //                 last_slide = Some(SlideDir::Right);
    //             }
    //             HumanInputEvent::NavDown => {
    //                 last_slide = Some(SlideDir::Left);
    //             }
    //             HumanInputEvent::OkPressed => {
    //                 toggle_view = true;
    //             }
    //             other => {
    //                 // Forward input to current app
    //                 let mut comp = compositor.lock().await;
    //                 let current = comp.current_handle();
    //                 if let Some(window) = comp.get_window_mut(current) {
    //                     let _ = window.input_sender().try_send(other);
    //                 }
    //             }
    //         }
    //     }
    //
    //     // Apply slide or view toggle logic
    //     if let Some(dir) = last_slide {
    //         let mut comp = compositor.lock().await;
    //         comp.animate_slide(dir).await;
    //         comp.step().await;
    //     } else if toggle_view {
    //         let mut comp = compositor.lock().await;
    //         comp.toggle_view();
    //         let current = comp.current_handle();
    //         comp.request_redraw(current);
    //         comp.step().await;
    //     } else {
    //         // No user input: step once every ~100ms for idle refresh
    //         Timer::after(Duration::from_millis(100)).await;
    //         let mut comp = compositor.lock().await;
    //         let current = comp.current_handle();
    //         comp.request_redraw(current); // passive draw (for e.g. clock, sensor UI)
    //         comp.step().await;
    //     }
    //
    //     // Sleep remaining time if loop was too fast (ensure ~10Hz)
    //     let elapsed = Instant::now() - idle_start;
    //     if elapsed < Duration::from_millis(30) {
    //         Timer::after(Duration::from_millis(100) - elapsed).await;
    //     }
    // }
}
