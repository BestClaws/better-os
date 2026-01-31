use crate::system::hal::vibrator::AsyncVibrator;
use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

// TODO: hard assuming we are using ADC1, bad. even for a driver.
pub struct VibratorDriver<'a> {
    vibrator_pin: Output<'a>,
}

impl<'a> VibratorDriver<'a> {
    pub fn new(vibrator_pin: Output<'a>) -> Self {
        Self { vibrator_pin }
    }
}

#[async_trait(?Send)]
impl<'a> AsyncVibrator for VibratorDriver<'a> {
    async fn vibrate(&mut self, duration: Duration) {
        self.vibrator_pin.set_high();
        Timer::after(duration).await;
        self.vibrator_pin.set_low();
    }
}
