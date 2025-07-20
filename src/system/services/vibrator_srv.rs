use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use crate::system::hal::vibrator::AsyncVibrator;

pub static VIBRATION_SIG: Signal<CriticalSectionRawMutex, Duration> = Signal::new();





#[embassy_executor::task]
pub async fn vibrator_service(vibrator: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncVibrator>>) {

    let mut vibrator = vibrator.lock().await;
    loop {
        vibrator.vibrate(VIBRATION_SIG.wait().await).await;

    }

}
