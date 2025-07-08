use alloc::boxed::Box;
       // v1.0.0
      // v1.0.0
use async_trait::async_trait;
use defmt::Format;




#[async_trait(?Send)]
pub trait AsyncGyroAccelerometer {
    async fn calibrate(&mut self);

    async fn get_accelerometer_data(&mut self) -> (i16, i16, i16);
    async fn get_gyroscope_data(&mut self) -> (i16, i16, i16);
    async fn get_temperature_celsius(&mut self) -> u8;
}

