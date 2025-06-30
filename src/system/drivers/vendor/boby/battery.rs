use core::cell::{Cell};
use defmt::{info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcConfig, AdcPin, Attenuation};
use esp_hal::peripherals::{ADC1, GPIO2};

pub static BATTERY_READ_SIG: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub static  BATTERY_PERCENT: Mutex<CriticalSectionRawMutex, Cell<u16>> = Mutex::new(Cell::new(0));


#[embassy_executor::task]
pub async fn battery_task(
    per_adc: ADC1<'static>,
    pin_batt: GPIO2<'static>,
) {
    let mut adc_config = AdcConfig::new();
    let mut adc_pin = adc_config.enable_pin(pin_batt, Attenuation::_11dB);
    let mut adc = Adc::new(per_adc, adc_config);
    let batt_pin_reading: u16 = nb::block!(adc.read_oneshot(&mut adc_pin)).unwrap();
    info!("[main] ADC read value: {}", batt_pin_reading);

    BATTERY_READ_SIG.wait().await;
    let reading = match nb::block!(adc.read_oneshot(&mut adc_pin)) {
        Ok(adc_reading) => {
            adc_reading
        }
        Err(_) => {
            warn!("[battery] Failed to read ADC");
            0
        }
    };

    BATTERY_PERCENT.lock().await.replace(reading);
}
pub async fn battery_percent() -> u16 {
    // give a chance to THE battery task to populate data
    BATTERY_READ_SIG.signal(());
    Timer::after(Duration::from_micros(1)).await;
     BATTERY_PERCENT.lock().await.get()

}