use alloc::boxed::Box;
use crate::system::hal::battery::AsyncBattery;
use async_trait::async_trait;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::analog::adc::{Adc, AdcPin};
use esp_hal::peripherals::{ADC1, GPIO1, GPIO3};
use esp_hal::Async;
use esp_hal::interrupt::map;
use crate::system::hal::ambient_sensor::AsyncAmbientSensor;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;

// TODO: hard assuming we are using ADC1, bad. even for a driver.
pub struct AmbientSensorDriver {
    adc: &'static  Mutex<CriticalSectionRawMutex, Adc<'static, ADC1<'static>, Async>>,
    sensor_pin: AdcPin<GPIO3<'static>, ADC1<'static>>
}

impl AmbientSensorDriver {
    pub fn new(adc: &'static  Mutex<CriticalSectionRawMutex, Adc<'static, ADC1<'static>, Async>>, sensor_pin: AdcPin<GPIO3<'static>, ADC1<'static>>) -> Self {
        Self { adc,  sensor_pin }
    }
}


#[async_trait(?Send)]
impl AsyncAmbientSensor for AmbientSensorDriver {
   
    async fn percent(&mut self) -> u8 {
        let v = self.adc.lock().await.read_oneshot(&mut self.sensor_pin).await;
        let percent = (v as f32 / 4095.0) * 100.0;
        percent as u8
    }
}
