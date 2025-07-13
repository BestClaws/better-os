use alloc::boxed::Box;
use core::fmt::Debug;
use async_trait::async_trait;
use defmt::export::u8;
use defmt::{info, println};
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
    gyroscopeResolution: f32,
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {





    }



    async fn get_orientation(&mut self) -> Quaternion {
        todo!()
    }
}


impl<I> MPU6050<I> where I: I2c
{
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            address: MPU6050_DEFAULT_ADDRESS,
            gyroscopeResolution: 2000.0 / 32768.0
        }
    }


    pub async fn initialize(&mut self) {
        self.set_clock_source(MPU6050_CLOCK_PLL_XGYRO).await;
        self.set_full_scale_gyro_range(MPU6050_GYRO_FS_250).await;

    }


    async fn set_full_scale_gyro_range(&mut self, mut range: u8) {

        match range {
            MPU6050_GYRO_FS_250 =>
                self.gyroscopeResolution = 250.0 / 32768.0,
            MPU6050_GYRO_FS_500 =>
                self.gyroscopeResolution = 500.0 / 32768.0,
            MPU6050_GYRO_FS_1000 =>
                self.gyroscopeResolution = 1000.0 / 32768.0,
            MPU6050_GYRO_FS_2000 =>
                self.gyroscopeResolution = 2000.0 / 32768.0,
            _ => {
                info!("Init gyroRange not valid, setting maximum gyro range");
                range = MPU6050_GYRO_FS_2000;
                self.gyroscopeResolution = 2000.0 / 32768.0;
            }
        }


        self.write_bits(self.address, MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, &[range]).await;



        }

    async fn set_clock_source(&mut self, source: u8)  {
        self.write_bits(self.address, MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, &[source]).await;
    }
}







impl<'a, I> I2cHelpers<'a, I> for MPU6050<I>
where
    I: I2c + 'a,
{
    async fn read_bit(
        &mut self,
        address: u8,
        register: u8,
        bit_num: u8,
        data: &mut [u8],
        _timeout: Duration,
    ) -> Result<u8, I::Error> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(address, &[register], &mut buf).await?;
        data[0] = (buf[0] >> bit_num) & 0x01;
        Ok(1)
    }

    async fn read_bits(
        &mut self,
        address: u8,
        register: u8,
        bit_start: u8,
        length: u8,
        data: &mut [u8],
        _timeout: Duration,
    ) -> Result<u8, I::Error> {
        let mut b = [0u8; 1];
        self.i2c.write_read(address, &[register], &mut b).await?;
        let mask = ((1 << length) - 1) << (bit_start - length + 1);
        data[0] = (b[0] & mask) >> (bit_start - length + 1);
        Ok(1)
    }

    async fn read_byte(
        &mut self,
        address: u8,
        register: u8,
        data: &mut [u8],
        timeout: Duration,
    ) -> Result<u8, I::Error> {
        self.read_bytes(address, register, 1, data, timeout).await
    }

    async fn read_word(
        &mut self,
        address: u8,
        register: u8,
        data: &mut [u16],
        timeout: Duration,
    ) -> Result<u8, I::Error> {
        self.read_words(address, register, 1, data, timeout).await
    }

    async fn read_bytes(
        &mut self,
        address: u8,
        register: u8,
        length: u8,
        data: &mut [u8],
        _timeout: Duration,
    ) -> Result<u8, I::Error> {
        self.i2c.write_read(address, &[register], &mut data[..length as usize]).await?;
        Ok(length)
    }

    async fn read_words(
        &mut self,
        address: u8,
        register: u8,
        length: u8,
        data: &mut [u16],
        _timeout: Duration,
    ) -> Result<u8, I::Error> {
        let mut buf = [0u8; 128];
        let byte_len = (length as usize) * 2;
        self.i2c.write_read(address, &[register], &mut buf[..byte_len]).await?;

        for i in 0..length as usize {
            data[i] = ((buf[2 * i] as u16) << 8) | buf[2 * i + 1] as u16;
        }
        Ok(length)
    }

    async fn write_bit(
        &mut self,
        address: u8,
        register: u8,
        bit_num: u8,
        data: &[u8],
    ) {
        let mut buf = [0u8; 1];
        if self.i2c.write_read(address, &[register], &mut buf).await.is_ok() {
            let b = if data[0] != 0 {
                buf[0] | (1 << bit_num)
            } else {
                buf[0] & !(1 << bit_num)
            };
            let _ = self.i2c.write(address, &[register, b]).await;
        }
    }

    async fn write_bits(
        &mut self,
        address: u8,
        register: u8,
        bit_start: u8,
        length: u8,
        data: &[u8],
    ) {
        let mut buf = [0u8; 1];
        if self.i2c.write_read(address, &[register], &mut buf).await.is_ok() {
            let mut b = buf[0];
            let mask = ((1 << length) - 1) << (bit_start - length + 1);
            let mut value = data[0] << (bit_start - length + 1);
            value &= mask;
            b &= !mask;
            b |= value;
            let _ = self.i2c.write(address, &[register, b]).await;
        }
    }

    async fn write_byte(
        &mut self,
        address: u8,
        register: u8,
        data: &[u8],
    ) {
        let _ = self.i2c.write(address, &[register, data[0]]).await;
    }

    async fn write_word(
        &mut self,
        address: u8,
        register: u8,
        data: &[u16],
    ) {
        let mut buf = [0u8; 2];
        buf[0] = (data[0] >> 8) as u8;
        buf[1] = (data[0] & 0xFF) as u8;
        let _ = self.i2c.write(address, &[register, buf[0], buf[1]]).await;
    }

    async fn write_bytes(
        &mut self,
        address: u8,
        register: u8,
        length: u8,
        data: &mut [u8],
    ) {
        let mut buf = [0u8; 128];
        buf[0] = register;
        buf[1..=length as usize].copy_from_slice(&data[..length as usize]);
        let _ = self.i2c.write(address, &buf[..=length as usize]).await;
    }

    async fn write_words(
        &mut self,
        address: u8,
        register: u8,
        length: u8,
        data: &mut [u16],
    ) {
        let mut buf = [0u8; 128];
        buf[0] = register;
        for i in 0..length as usize {
            buf[1 + 2 * i] = (data[i] >> 8) as u8;
            buf[1 + 2 * i + 1] = (data[i] & 0xFF) as u8;
        }
        let _ = self.i2c.write(address, &buf[..1 + (length as usize * 2)]).await;
    }
}
