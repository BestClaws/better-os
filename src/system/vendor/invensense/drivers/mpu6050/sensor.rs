use alloc::boxed::Box;
use core::fmt::Debug;
use async_trait::async_trait;
use defmt::{debug, error, info, unwrap};
use defmt::export::u8;
use embassy_time::{with_timeout, Duration, Timer, WithTimeout};
use embedded_hal_async::i2c::I2c;
use esp_hal::interrupt::map;
use log::__private_api::enabled;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::system::vendor::invensense::drivers::mpu6050::dmp_firmware::DMP_FIRMWARE;
use crate::util::math::primitives::{Quaternion, Vec3};
use micromath::F32Ext;


const MPU6050_DEFAULT_ADDRESS: u8 = 0x68; // Default I2C address for MPU6050

pub struct MPU6050<I> where I: I2c {
    i2c: I,
    address: u8,
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {


    }



    async fn get_orientation(&mut self) -> Quaternion {
        todo!()
    }
}


impl<I> MPU6050<I> where I: I2c {
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            address: MPU6050_DEFAULT_ADDRESS,
        }
    }





}