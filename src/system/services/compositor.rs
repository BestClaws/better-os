use alloc::boxed::Box;
use defmt::{info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Instant, Timer};
use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::{HumanInputEvent, INPUT_CHANNEL};

// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn compositor_service(x: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>) {
    info!("[{}s] compositor  service started", Instant::now().as_millis() as f32 / 1000f32);
    let mut d = x.lock().await;
    d.init().await;






    loop {
        let event = INPUT_CHANNEL.receive().await;
        INPUT_CHANNEL.clear();

        match event {
            HumanInputEvent::NavUp => {
                info!("Received NavUp event");
                d.draw(0).await;
            },
            HumanInputEvent::NavDown => {
                info!("Received NavDown event");
                d.draw(2).await;
            },
            HumanInputEvent::OK_PRESSED => {
                info!("Received OK_PRESSED event");
                d.draw(3).await;
            },
            HumanInputEvent::OK_RELEASED => {
                info!("Received OK_RELEASED event");
                d.draw(4).await;
            },
            HumanInputEvent::OK_HELD => {
                info!("Received OK_HELD event");
                d.draw(5).await;
            },
        };

        Timer::after_millis(1).await;


    }

    // error!("Compositor service stopped");

}





