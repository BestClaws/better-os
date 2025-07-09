use alloc::boxed::Box;
use defmt::{info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, INPUT_CHANNEL};

// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn compositor_service(x: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>) {

    Timer::after_secs(1000).await;

    info!("[{}s] compositor  service started", Instant::now().as_millis() as f32 / 1000f32);
    let mut d = x.lock().await;
    d.init().await;






    loop {
        let event = INPUT_CHANNEL.receive().with_timeout(Duration::from_millis(20)).await;
        INPUT_CHANNEL.clear();

        match event {
            Ok(event) => {
                d.draw(event).await;

            },
            Err(_) => {
                // No event received, continue the loop
                d.draw(HumanInputEvent::OkHeld).await;
                continue;
            }
        };




        Timer::after_millis(1).await;


    }


    // error!("Compositor service stopped");

}





