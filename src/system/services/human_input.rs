use alloc::boxed::Box;
use defmt::{info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use crate::system::hal::encoder::AsyncEncoder;



// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn human_input_service(x: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>) {
    info!("[{}s] human input service started", Instant::now().as_millis() as f32 / 1000f32);

    loop {
        let Ok(success) = x.lock().await.next().await else {
            info!("invalid encoder state");
            continue;
        };
        info!("detected input: {}", success);

    }
    // error!("human input service stopped");

}





