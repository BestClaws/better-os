use core::cell::Cell;
use defmt::{info, Debug2Format};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::GPIO7;

pub static VIBRATION_PERIOD_UPDATE_SIG: Signal<CriticalSectionRawMutex, Duration> = Signal::new();
pub static VIBRATION_SIG: Signal<CriticalSectionRawMutex, Duration> = Signal::new();


#[embassy_executor::task]
pub async fn periodic_vibration() {


    let mut period = Cell::new(Duration::from_millis(5000));
    loop {

        select(
            async {
                VIBRATION_SIG.signal(Duration::from_millis(500));
                let d = period.get();
                Timer::after(d).await;

            },
            async {
                // maybe calculate the elapsed duration so far and calculate the remaining duration to wait
                // before the next vibration, so the new vibration period is honored
                period.set(VIBRATION_PERIOD_UPDATE_SIG.wait().await);
                info!("VIBRATION_PERIOD_UPDATE_SIG");
            }
        ).await;

        Timer::after(Duration::from_millis(1000)).await;

    }
}

#[embassy_executor::task]
pub async fn vibrator(pin: GPIO7<'static>) {
    let vibration_len = Cell::new(Duration::from_millis(500));
    let mut vibrator = Output::new(pin, Level::Low, OutputConfig::default());

    loop {
        select(
            async {
                vibration_len.set(VIBRATION_SIG.wait().await);
                vibrator.set_high();
                Timer::after(vibration_len.get()).await;
                vibrator.set_low();
            },
            async {
                vibration_len.set(VIBRATION_PERIOD_UPDATE_SIG.wait().await);
            }

        ).await;

        Timer::after(Duration::from_millis(1000)).await;
    }




}

