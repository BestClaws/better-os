use alloc::boxed::Box;
use core::cell::Cell;
use defmt::{info, Debug2Format};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Level, Output, OutputConfig};
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::vibrator::AsyncVibrator;

pub static VIBRATION_PERIOD_UPDATE_SIG: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static VIBRATION_SIG: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


#[embassy_executor::task]
pub async fn periodic_vibration() {
    let period = Cell::new(Duration::from_millis(5000));

    loop {

        select(
            async {
                VIBRATION_SIG.signal(Duration::from_millis(500));
                Timer::after(period.get()).await;

            },
            async {
                // maybe calculate the elapsed duration so far and calculate the remaining duration to wait
                // before the next vibration, so the new vibration period is honored
                period.set(VIBRATION_PERIOD_UPDATE_SIG.wait().await);
                info!("VIBRATION_PERIOD_UPDATE_SIG: {}", Debug2Format(&period.get()));
            }
        ).await;

        Timer::after(Duration::from_millis(50)).await;

    }
}

#[embassy_executor::task]
pub async fn vibrator_service(vibrator: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncVibrator>>) {

    let mut vibrator = vibrator.lock().await;
    loop {
        // vibrator.vibrate(Duration::from_millis(500)).await;
        Timer::after(Duration::from_secs(5)).await;
    }

}
