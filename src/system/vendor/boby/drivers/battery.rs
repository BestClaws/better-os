use alloc::boxed::Box;
use crate::system::hal::battery::AsyncBattery;
use async_trait::async_trait;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::analog::adc::{Adc, AdcPin};
use esp_hal::peripherals::{ADC1, GPIO1, GPIO3};
use esp_hal::Async;
use esp_hal::interrupt::map;

// TODO: hard assuming we are using ADC1, bad. even for a driver.
pub struct BatteryDriver {
    adc: &'static  Mutex<CriticalSectionRawMutex, Adc<'static, ADC1<'static>, Async>>,
    batt_pin: AdcPin<GPIO1<'static>, ADC1<'static>>
}

impl BatteryDriver {
    pub fn new(adc: &'static  Mutex<CriticalSectionRawMutex, Adc<'static, ADC1<'static>, Async>>, batt_pin: AdcPin<GPIO1<'static>, ADC1<'static>>) -> Self {
        Self { adc,  batt_pin }
    }
}


#[async_trait(?Send)]
impl AsyncBattery for BatteryDriver {
   

    async fn percent(&mut self) -> u8 {
        let v = self.adc.lock().await.read_oneshot(&mut self.batt_pin).await;
        let percent = (v as f32 / 4095.0) * 100.0;
        percent as u8
    }
}
