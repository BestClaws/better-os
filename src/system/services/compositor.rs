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

        d.draw(event).await;



        Timer::after_millis(1).await;


    }

    // error!("Compositor service stopped");

}





