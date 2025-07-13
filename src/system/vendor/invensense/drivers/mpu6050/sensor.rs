use alloc::boxed::Box;
use core::fmt::Debug;
use async_trait::async_trait;
use defmt::export::u8;
use embassy_time::{with_timeout, Duration, Timer, WithTimeout};
use embedded_hal_async::i2c::I2c;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use micromath::F32Ext;
use crate::system::vendor::invensense::drivers::mpu6050::error::Error;
use crate::system::vendor::invensense::drivers::mpu6050::registers::*;
use crate::system::vendor::invensense::drivers::mpu6050::constants::*;
use crate::system::vendor::invensense::drivers::mpu6050::i2c_helpers::I2cHelpers;

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


    pub async fn initialize(&mut self) {
        self.set_clock_source(MPU6050_CLOCK_PLL_XGYRO).await.unwrap();
    }

    async fn set_clock_source(&mut self, source: u8) -> Result<(), Error<I>> {
        // Implementation for setting the clock source
        Ok(())
    }







}

impl<I> I2cHelpers<I> for MPU6050<I> where I: I2c {
    fn write_bits(&mut self, address: u8, register: u8, bitStart: u8, length: u8, mut data: u8) -> Result<(), I::Error> {
        let mut b: u8 = 0;
        if (readByte(devAddr, regAddr, &b, I2Cdev::readTimeout, wireObj) != 0) {
            let mask: u8  = ((1 << length) - 1) << (bitStart - length + 1);
            data <<= (bitStart - length + 1); // shift data into correct position
            data &= mask; // zero all non-important bits in data
            b &= !(mask); // zero all important bits in existing byte
            b |= data; // combine data with existing byte
            return writeByte(devAddr, regAddr, b, wireObj);
        } else {
            return Ok(false);
        }
    }
}