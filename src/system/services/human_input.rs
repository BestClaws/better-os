use alloc::boxed::Box;
use defmt::{error, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Duration;
use crate::system::hal::encoder::AsyncEncoder;

pub static SIG_A: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static SIG_B: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn human_input_service(x: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>) {
    info!("human input  service started");
    loop {
        x.lock().await.next().await.unwrap();
        info!("detected input");

    }
    error!("human input service stopped");

}





