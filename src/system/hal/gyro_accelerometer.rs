use alloc::boxed::Box;
       // v1.0.0
      // v1.0.0
use async_trait::async_trait;
use defmt::{info, Format};
use mpu6050_dmp::quaternion::Quaternion;
use mpu6050_dmp::yaw_pitch_roll::YawPitchRoll;

#[async_trait(?Send)]
pub trait AsyncGyroAccelerometer {
    async fn calibrate(&mut self);

    async fn get_accelerometer_data(&mut self) -> (f32, f32, f32);
    async fn get_gyroscope_data(&mut self) -> (f32, f32, f32);
    async fn get_temperature_celsius(&mut self) -> u8;
    async fn pitch_roll_yaw(&mut self) -> (i16, i16, i16);

}

