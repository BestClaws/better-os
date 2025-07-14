use alloc::boxed::Box;
use core::fmt::Debug;
use async_trait::async_trait;
use defmt::export::u8;
use defmt::{error, info, println, warn};
use embassy_time::{with_timeout, Duration, Timer, WithTimeout};
use embedded_hal_async::i2c::I2c;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use micromath::F32Ext;
use crate::system::vendor::invensense::drivers::mpu6050::error::Error;
use crate::system::vendor::invensense::drivers::mpu6050::constants::*;
use crate::system::vendor::invensense::drivers::mpu6050::i2c_helpers::I2cHelpers;

const MPU6050_DEFAULT_ADDRESS: u8 = 0x68; // Default I2C address for MPU6050
const TIMEOUT: Duration = Duration::from_millis(1000);

pub struct MPU6050<I> where I: I2c {
    i2c: I,
    address: u8,
    gyroscope_resolution: f32,
    acceleration_resolution: f32,
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {
        self.initialize().await;
        self.test_connection().await.unwrap();
        self.dmp_initialize().await.unwrap();

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
            gyroscope_resolution: 2000.0 / 32768.0,
            acceleration_resolution: 16.0 / 32768.0,
        }
    }


    async fn dmp_initialize(&mut self) -> Result<(), Error<I>> {
        info!("resetting MPU");
        self.reset_device().await;
        self.set_sleep_enabled(false).await;
        self.set_memory_bank(0x10, true, true).await;
        self.set_memory_start_address(0x06).await;
        info!("checking HW revision.");
        // don't read this again, doing so will change its value.
        let rev = self.read_memory_byte().await?;
        info!("Revision @ user[16][6] = {}", rev);




        Ok(())
    }


    async fn read_memory_byte(&mut self) -> Result<u8, Error<I>> {
        let  buffer = &mut [0u8];
        self.read_byte(self.address, MPU6050_RA_MEM_R_W, buffer, TIMEOUT).await.map_err(|e| Error::I2cError(e))?;
        Ok(buffer[0])
    }
    async fn set_memory_start_address(&mut self, address: u8) {
        self.write_byte(self.address, MPU6050_RA_MEM_START_ADDR, address).await;
    }


    async fn  set_memory_bank(&mut self, mut bank: u8, prefetch_enabled: bool, user_bank: bool) {
        bank &= 0x1F;
        if user_bank {bank |= 0x20};
        if prefetch_enabled { bank |= 0x40};
        self.write_byte(self.address, MPU6050_RA_BANK_SEL, bank).await;
    }


    async fn reset_device(&mut self) {
        // don't try to read this again. as it will always return 0x00
        self.write_bit(self.address, MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_DEVICE_RESET_BIT, true as u8).await;
        Timer::after(Duration::from_millis(50)).await; // Wait for reset to complete
    }

    async fn initialize(&mut self) {
        self.set_clock_source(MPU6050_CLOCK_PLL_XGYRO).await;
        self.set_full_scale_gyro_range(MPU6050_GYRO_FS_250).await;
        self.set_full_scale_accel_range(MPU6050_ACCEL_FS_2).await;
        self.set_sleep_enabled(false).await;


    }

    async fn test_connection(&mut self) -> Result<(), Error<I>> {
        let device_id = self.get_device_id().await?;
        if  (device_id == 0x34) || (device_id == 0xC) || (device_id == 0x3A) {
            Ok(())
        } else {
            Err(Error::WrongDevice)
        }

    }

    async fn get_device_id(&mut self) -> Result<u8, Error<I>> {
        let buffer = &mut [0];
        self.read_bits(self.address, MPU6050_RA_WHO_AM_I, MPU6050_WHO_AM_I_BIT, MPU6050_WHO_AM_I_LENGTH, buffer, Duration::from_millis(100))
            .await.map_err(|e| Error::I2cError(e))?;
        Ok(buffer[0])
    }


    async fn set_sleep_enabled(&mut self, enabled: bool) {
        self.write_bit(self.address, MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_SLEEP_BIT, enabled as u8).await;
    }


    async fn set_full_scale_gyro_range(&mut self, mut range: u8) {

        match range {
            MPU6050_GYRO_FS_250 =>
                self.gyroscope_resolution = 250.0 / 32768.0,
            MPU6050_GYRO_FS_500 =>
                self.gyroscope_resolution = 500.0 / 32768.0,
            MPU6050_GYRO_FS_1000 =>
                self.gyroscope_resolution = 1000.0 / 32768.0,
            MPU6050_GYRO_FS_2000 =>
                self.gyroscope_resolution = 2000.0 / 32768.0,
            _ => {
                info!("Init gyroRange not valid, setting maximum gyro range");
                range = MPU6050_GYRO_FS_2000;
                self.gyroscope_resolution = 2000.0 / 32768.0;
            }
        }
        self.write_bits(self.address, MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, &[range]).await;

    }

    async fn set_full_scale_accel_range(&mut self, mut range: u8) {

        match range {
            MPU6050_ACCEL_FS_2 =>
                self.acceleration_resolution = 2.0 / 32768.0,
            MPU6050_ACCEL_FS_4 =>
                self.acceleration_resolution  = 4.0 / 32768.0,
            MPU6050_ACCEL_FS_8 =>
                self.acceleration_resolution  = 8.0 / 32768.0,
            MPU6050_ACCEL_FS_16 =>
                self.acceleration_resolution  = 16.0 / 32768.0,
            _ => {
                info!("Init accelRange not valid, setting maximum accel range");
                range = MPU6050_ACCEL_FS_16;
                self.acceleration_resolution  = 16.0 / 32768.0;
            }
        }
        self.write_bits(self.address, MPU6050_RA_ACCEL_CONFIG, MPU6050_ACONFIG_AFS_SEL_BIT, MPU6050_ACONFIG_AFS_SEL_LENGTH, &[range]).await;

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
        let mask = ((1 << length) - 1) << (1 + bit_start  - length);
        data[0] = (b[0] & mask) >> (1 + bit_start - length);
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

    // todo: return error
    async fn write_bit(
        &mut self,
        address: u8,
        register: u8,
        bit_num: u8,
        data: u8,
    ) {
        let mut buf = [0u8; 1];
        if self.i2c.write_read(address, &[register], &mut buf).await.is_ok() {
            let b= if data != 0 {
                buf[0] | (1 << bit_num)
            } else {
                buf[0] & !(1 << bit_num)
            };
            self.i2c.write(address, &[register, b]).await.unwrap();
        }
        // // read bit again to verify
        // let mut read_buf = [0u8; 1];
        // if self.read_bit(address, register, bit_num, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
        //     if read_buf[0] != data {
        //         error!("Error in written bit: expected {}, got {}", data, read_buf[0]);
        //     }
        // } else {
        //     warn!("Failed to read back written bit");
        // }
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
            let mask = ((1 << length) - 1) << (1 + bit_start - length);
            let mut value = data[0] << (1 + bit_start - length);
            value &= mask;
            b &= !mask;
            b |= value;
            let _ = self.i2c.write(address, &[register, b]).await;

            // // verify
            // let mut read_buf = [0u8; 1];
            // if self.read_bits(address, register, bit_start, length, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
            //     let expected_value = (data[0] << (1 + bit_start - length)) & mask;
            //     if (read_buf[0] & mask) != expected_value {
            //         error!("Error in written bits: expected {}, got {}", expected_value, read_buf[0] & mask);
            //     }
            // } else {
            //     warn!("Failed to read back written bits");
            // }
        } else {
            error!("Failed to read register {} at address {} so as to  modify bits", register, address);
        }


    }

    async fn write_byte(
        &mut self,
        address: u8,
        register: u8,
        data: u8,
    ) {
        self.i2c.write(address, &[register, data]).await.unwrap();
        Timer::after_millis(1000).await;
        // // verify
        // let mut read_buf = [0u8; 1];
        // if self.read_byte(address, register, &mut read_buf, Duration::from_millis(1000)).await.is_ok() {
        //     if read_buf[0] != data {
        //         error!("Error in written byte: expected {}, got {}", data, read_buf[0]);
        //     } else {
        //         info!("write byte succcess. got: {}, expected: {}", read_buf[0], data);
        //     }
        // } else {
        //     warn!("Failed to read back written byte");
        // }

    }

    async fn write_word(
        &mut self,
        address: u8,
        register: u8,
        data: u16,
    ) {
        let mut buf = [0u8; 2];
        buf[0] = (data >> 8) as u8;
        buf[1] = (data & 0xFF) as u8;
        let _ = self.i2c.write(address, &[register, buf[0], buf[1]]).await;

        // // verify
        // let mut read_buf = [0u16; 1];
        // if self.read_word(address, register, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
        //     let expected_value = data;
        //     let read_value = read_buf[0];
        //     if read_value != expected_value {
        //         error!("Error in written word: expected {}, got {}", expected_value, read_value);
        //     }
        // } else {
        //     warn!("Failed to read back written word");
        // }
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

        // // verify
        // let mut read_buf = [0u8; 128];
        // if self.read_bytes(address, register, length, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
        //     if read_buf[..length as usize] != data[..length as usize] {
        //         error!("Error in written bytes: expected {:?}, got {:?}", &data[..length as usize], &read_buf[..length as usize]);
        //     }
        // } else {
        //     warn!("Failed to read back written bytes");
        // }
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

        // // verify
        // let mut read_buf = [0u16; 128];
        // if self.read_words(address, register, length, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
        //     for i in 0..length as usize {
        //         if read_buf[i] != data[i] {
        //             error!("Error in written word {}: expected {}, got {}", i, data[i], read_buf[i]);
        //         }
        //     }
        // } else {
        //     warn!("Failed to read back written words");
        // }
    }
}
