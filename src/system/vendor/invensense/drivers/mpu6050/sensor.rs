use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::vendor::invensense::drivers::mpu6050::constants::*;
use crate::system::vendor::invensense::drivers::mpu6050::dmp_firmware::DMP_FIRMWARE;
use crate::system::vendor::invensense::drivers::mpu6050::error::Error;
use crate::system::vendor::invensense::drivers::mpu6050::i2c_helpers::I2cHelpers;
use alloc::boxed::Box;
use core::f32::consts::PI;
use async_trait::async_trait;
use defmt::{error, info};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;
#[allow(unused_imports)]
use micromath::F32Ext;
use crate::util::math::primitives::{Quaternion, Vec3};
use libm::{atan2f, sqrtf};
const MPU6050_DEFAULT_ADDRESS: u8 = 0x68; // Default I2C address for MPU6050
const TIMEOUT: Duration = Duration::from_millis(1000);

pub struct MPU6050<I> where I: I2c {
    i2c: I,
    address: u8,
    gyroscope_resolution: f32,
    acceleration_resolution: f32,
    dmp_packet_size: u16,
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {
        self.initialize().await;
        self.test_connection().await.unwrap();
        self.dmp_initialize().await.unwrap();
        // TODO: should be done after calibrating
        self.set_dmp_enabled(true).await;

    }



    async fn get_orientation(&mut self) -> Quaternion {
        loop {
            let packet_size = self.get_fifo_packet_size().await;
            let Ok(mut fifo_count) = self.get_fifo_count().await else {
                Timer::after(Duration::from_millis(10)).await;
                continue;
            };
            let buffer = &mut [0u8; 64];

            if fifo_count >= packet_size {
                // Keep the latest complete packet
                while fifo_count > packet_size {
                    let Ok(_) = self.get_fifo_bytes(buffer, packet_size as u8).await else {
                        Timer::after_millis(10).await;
                        continue;
                    };
                    fifo_count -= packet_size;
                }
                return get_orientation_from_fifo_bytes(buffer).await;
            }
            Timer::after_millis(50).await;// Sample rate ~20Hz
        }
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
            dmp_packet_size: 42, // Default DMP packet size
        }
    }


    async fn dmp_initialize(&mut self) -> Result<(), Error<I>> {
        info!("resetting MPU");
        self.reset_device().await;
        self.set_sleep_enabled(false).await;
        self.set_memory_bank(0x10, true, true).await;
        self.set_memory_start_address(0x06).await;
        // don't read this again, doing so will change its value.
        let rev = self.read_memory_byte().await?;
        info!("HW Revision @ user[16][6] = {}", rev);
        info!("Resetting memory bank selection to 0...");
        self.set_memory_bank(0, false, false).await;
        info!("reading OTP bank validity... OTP Bank Valid: {}", self.get_otp_bank_valid().await?);

        // setup weird slave stuff
        // setup weird slave stuff (?)
        info!("Setting slave 0 address to 0x7F...");
        self.set_slave_address(0, 0x7F).await;
        info!("Disabling I2C Master mode...");
        self.set_i2c_master_mode_enabled(false).await;
        info!("Setting slave 0 address to 0x68 (self)...");
        self.set_slave_address(0, 0x68).await;
        info!("Resetting I2C Master control...");
        self.reset_i2c_master().await;
        info!("settings clock source to z gyro");
        self.set_clock_source(MPU6050_CLOCK_PLL_ZGYRO).await;
        info!("Setting DMP and FIFO_OFLOW interrupts enabled...");
        self.set_int_enabled(1<<MPU6050_INTERRUPT_FIFO_OFLOW_BIT|1<<MPU6050_INTERRUPT_DMP_INT_BIT).await;
        info!("Setting sample rate to 200Hz...");
        self.set_rate(4).await; // 1khz / (1 + 4) = 200 Hz
        info!("Setting external frame sync to TEMP_OUT_L[0]...");
        self.set_external_frame_sync(MPU6050_EXT_SYNC_TEMP_OUT_L).await;
        info!("Setting DLPF bandwidth to 42Hz...");
        self.set_dlpf_mode(MPU6050_DLPF_BW_42).await;
        info!("Setting gyro sensitivity to +/- 2000 deg/sec...");
        self.set_full_scale_gyro_range(MPU6050_GYRO_FS_2000).await;

        // load DMP code into memory banks
        info!("Writing DMP code to MPU memory banks ({}) bytes", MPU6050_DMP_CODE_SIZE);
        if self.write_program_memory_block(DMP_FIRMWARE, MPU6050_DMP_CODE_SIZE, 0, 0, true).await.is_err() {
            error!("Failed to write DMP code to MPU memory banks");
            return Err(Error::FirmwareUploadVerificationFailed);
        } // Failed
        info!("Success! DMP code written and verified.");

        // Set the FIFO Rate Divisor into the DMP Firmware Memory
        let dmp_update = [0x00, MPU6050_DMP_FIFO_RATE_DIVISOR];
        self.write_memory_block(&dmp_update, 0x02, 0x02, 0x16, true).await.expect("dmp update"); // Lets write the dmpUpdate data to the Firmware image, we have 2 bytes to write in bank 0x02 with the Offset 0x16

        //write start address MSB into register
        self.set_dmp_config1(0x03).await;
        //write start address LSB into register
        self.set_dmp_config2(0x00).await;

        info!("Clearing OTP Bank flag...");
        self.set_otp_bank_valid(false).await;

        info!("Setting motion detection threshold to 2...");
        self.set_motion_detection_threshold(2).await;
        info!("Setting zero-motion detection threshold to 156...");
        self.set_zero_motion_detection_threshold(156).await;

        info!("Setting motion detection duration to 80...");
        self.set_motion_detection_duration(80).await;
        info!("Setting zero-motion detection duration to 0...");
        self.set_zero_motion_detection_duration(0).await;
        info!("Enabling FIFO...");
        self.set_fifo_enabled(true).await;
        info!("resetting DMP");
        self.reset_dmp().await;
        info!("DMP is good to go!");
        info!("Disabling DMP (you turn it on later)...");
        self.set_dmp_enabled(false).await;
        info!("Setting up internal 42-byte (default) DMP packet buffer...");
        self.dmp_packet_size = 42;
        info!("Resetting FIFO and clearing INT status one last time...");
        self.reset_fifo().await;
        self.get_int_status().await?;
        Ok(())
    }


    async fn get_fifo_count(&mut self) -> Result<u16, Error<I>> {
        let buffer = &mut [0u8; 2];
        self.read_bytes(self.address, MPU6050_RA_FIFO_COUNTH, 2, buffer, TIMEOUT).await.map_err(Error::I2cError)?;
        Ok(((buffer[0] as u16) << 8) | buffer[1] as u16)
    }

    async fn get_fifo_bytes(&mut self, data: &mut [u8], length: u8) -> Result<(), Error<I>> {
        if length > 0 {
            self.read_bytes(self.address, MPU6050_RA_FIFO_R_W, length, data, TIMEOUT).await.map_err(Error::I2cError)?;
        } else {
            data.fill(0);
        }
        Ok(())
    }

    async fn get_fifo_packet_size(&self) -> u16 {
        self.dmp_packet_size
    }

    async fn get_int_status(&mut self) -> Result<u8, Error<I>> {
        let buffer = &mut [0];
        self.read_byte(self.address, MPU6050_RA_INT_STATUS, buffer, TIMEOUT).await.map_err(Error::I2cError)?;
        Ok(buffer[0])
    }

    async fn reset_fifo(&mut self) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_FIFO_RESET_BIT, true as u8).await;
    }

    async fn set_dmp_enabled(&mut self, enabled: bool) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_DMP_EN_BIT, enabled as u8).await;
    }

    async fn reset_dmp(&mut self) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_DMP_RESET_BIT, true as u8).await;
    }

    async fn set_fifo_enabled(&mut self, enabled: bool) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_FIFO_EN_BIT, enabled as u8).await;
    }

    async fn set_zero_motion_detection_duration(&mut self, duration: u8) {
        self.write_byte(self.address, MPU6050_RA_ZRMOT_DUR, duration).await;
    }

    async fn set_motion_detection_duration(&mut self, duration: u8) {
        self.write_byte(self.address, MPU6050_RA_MOT_DUR, duration).await;
    }

    async fn set_zero_motion_detection_threshold(&mut self, threshold: u8) {
        self.write_byte(self.address, MPU6050_RA_ZRMOT_THR, threshold).await;
    }

    async fn set_motion_detection_threshold(&mut self, threshold: u8) {
        self.write_byte(self.address, MPU6050_RA_MOT_THR, threshold).await;
    }

    async fn set_otp_bank_valid(&mut self, enabled: bool) {
        self.write_bit(self.address, MPU6050_RA_XG_OFFS_TC, MPU6050_TC_OTP_BNK_VLD_BIT, enabled as u8).await;
    }


    async fn set_dmp_config1(&mut self, config: u8) {
        self.write_byte(self.address, MPU6050_RA_DMP_CFG_1, config).await;
    }

    async fn set_dmp_config2(&mut self, config: u8) {
        self.write_byte(self.address, MPU6050_RA_DMP_CFG_2, config).await;
    }


    async fn write_program_memory_block(&mut self, data: &[u8], data_size: u16, bank: u8, address: u8, verify: bool) -> Result<(), Error<I>> {
        self.write_memory_block(data, data_size, bank, address, verify).await
    }

    async fn write_memory_block(
        &mut self,
        data: &[u8],
        data_size: u16,
        mut bank: u8,
        mut address: u8,
        verify: bool,
    ) -> Result<(), Error<I>> {
        let mut i = 0;

        while i < data_size {
            // Determine chunk size
            let mut chunk_size = MPU6050_DMP_MEMORY_CHUNK_SIZE as u16;
            if i + chunk_size > data_size {
                chunk_size = data_size - i;
            }
            if chunk_size > (256 - address as u16) {
                chunk_size = 256 - address as u16;
            }

            // Get current chunk
            let chunk = &data[i as usize..(i + chunk_size) as usize];

            // Set memory bank/address
            self.set_memory_bank(bank, false, false).await;
            self.set_memory_start_address(address).await;

            // Write chunk
            let mut chunk_buf = [0u8; MPU6050_DMP_MEMORY_CHUNK_SIZE as usize];
            chunk_buf[..chunk_size as usize].copy_from_slice(chunk);
            self.write_bytes(
                self.address,
                MPU6050_RA_MEM_R_W,
                chunk_size as u8,
                &mut chunk_buf[..chunk_size as usize],
            ).await;

            // Verify chunk
            if verify {
                let mut verify_buf = [0u8; MPU6050_DMP_MEMORY_CHUNK_SIZE as usize];
                self.set_memory_bank(bank, false, false).await;
                self.set_memory_start_address(address).await;
                self.read_bytes(
                    self.address,
                    MPU6050_RA_MEM_R_W,
                    chunk_size as u8,
                    &mut verify_buf[..chunk_size as usize],
                    TIMEOUT,
                ).await.map_err(Error::I2cError)?;

                if &verify_buf[..chunk_size as usize] != chunk {
                    error!("Verification failed at bank {}, address {}", bank, address);
                    return Err(Error::FirmwareUploadVerificationFailed);
                }
            }

            i += chunk_size;
            address = address.wrapping_add(chunk_size as u8); // Wraps at 256

            if i < data_size && address == 0 {
                bank += 1;
            }
        }

        Ok(())
    }



    async fn set_dlpf_mode(&mut self, mode: u8) {
        self.write_bits(self.address, MPU6050_RA_CONFIG, MPU6050_CFG_DLPF_CFG_BIT, MPU6050_CFG_DLPF_CFG_LENGTH, mode).await;
    }

    async fn set_external_frame_sync(&mut self, sync: u8) {
        self.write_bits(self.address, MPU6050_RA_CONFIG, MPU6050_CFG_EXT_SYNC_SET_BIT, MPU6050_CFG_EXT_SYNC_SET_LENGTH, sync).await;
    }

    async fn set_rate(&mut self, rate: u8) {
        self.write_byte(self.address, MPU6050_RA_SMPLRT_DIV, rate).await;
    }

    async fn set_int_enabled(&mut self, enabled: u8) {
        self.write_byte(self.address, MPU6050_RA_INT_ENABLE, enabled).await;
    }


    async fn reset_i2c_master(&mut self) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_RESET_BIT, true as u8).await;
        Timer::after(Duration::from_millis(20)).await; // Wait for reset to complete
    }

    async fn set_i2c_master_mode_enabled(&mut self, enabled: bool) {
        self.write_bit(self.address, MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_EN_BIT, enabled as u8).await;
    }

    async fn set_slave_address(&mut self, num: u8, address: u8) {
        if num > 3 {
            return
        }
        self.write_byte(self.address, MPU6050_RA_I2C_SLV0_ADDR + num*3, address).await;
    }

    async fn get_otp_bank_valid(&mut self) -> Result<bool, Error<I>> {
        let buffer = &mut [0];
        self.read_bit(self.address, MPU6050_RA_XG_OFFS_TC, MPU6050_TC_OTP_BNK_VLD_BIT, buffer, TIMEOUT)
            .await.map_err(|e| Error::I2cError(e))?;
        Ok(buffer[0] != 0)
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
        self.write_bits(self.address, MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, range).await;

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
        self.write_bits(self.address, MPU6050_RA_ACCEL_CONFIG, MPU6050_ACONFIG_AFS_SEL_BIT, MPU6050_ACONFIG_AFS_SEL_LENGTH, range).await;

    }

    async fn set_clock_source(&mut self, source: u8)  {
        self.write_bits(self.address, MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, source).await;
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
        data: u8,
    ) {
        let mut buf = [0u8; 1];
        if self.i2c.write_read(address, &[register], &mut buf).await.is_ok() {
            let mut b = buf[0];
            let mask = ((1 << length) - 1) << (1 + bit_start - length);
            let mut value = data << (1 + bit_start - length);
            value &= mask;
            b &= !mask;
            b |= value;
            let _ = self.i2c.write(address, &[register, b]).await;

            // // verify
            // let mut read_buf = [0u8; 1];
            // if self.read_bits(address, register, bit_start, length, &mut read_buf, Duration::from_millis(10)).await.is_ok() {
            //     let expected_value = (data << (1 + bit_start - length)) & mask;
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



/// Calculates gravity vector from a quaternion.
/// The result represents the direction of gravity (down) in the device's frame.
pub fn get_gravity(q: &Quaternion) -> Vec3 {
    let x = 2.0 * (q.x * q.z - q.w * q.y);
    let y = 2.0 * (q.w * q.x + q.y * q.z);
    let z = q.w * q.w - q.x * q.x - q.y * q.y + q.z * q.z;

    Vec3(x, y, z)
}



/// Computes yaw (Z), pitch (Y), and roll (X) angles from a quaternion and gravity vector.
/// Returns a tuple: `(yaw, pitch, roll)` in **radians**.
pub fn get_yaw_pitch_roll(q: &Quaternion, gravity: &Vec3) -> (f32, f32, f32) {
    // Yaw (rotation around Z axis)
    let yaw = atan2f(
        2.0 * q.x * q.y - 2.0 * q.w * q.z,
        2.0 * q.w * q.w + 2.0 * q.x * q.x - 1.0,
    );

    // Pitch (nose up/down, rotation around Y axis)
    let mut pitch = atan2f(
        gravity.0, // x
        sqrtf(gravity.1 * gravity.1 + gravity.2 * gravity.2), // sqrt(y² + z²)
    );

    // Roll (tilt left/right, rotation around X axis)
    let roll = atan2f(gravity.1, gravity.2); // atan2(y, z)

    // Handle discontinuity when gravity.z is negative
    if gravity.2 < 0.0 {
        if pitch > 0.0 {
            pitch = PI - pitch;
        } else {
            pitch = -PI - pitch;
        }
    }

    (yaw, pitch, roll)
}



/// Extracts a Quaternion from a 14-byte DMP packet (MPU6050 FIFO output)
async fn get_orientation_from_fifo_bytes(fifo_bytes: &mut [u8]) -> Quaternion {
    // If `fifo_bytes` is too short, return identity quaternion (or handle as error)
    if fifo_bytes.len() < 14 {
        return Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }

    let q_i = [
        ((fifo_bytes[0] as i16) << 8) | fifo_bytes[1] as i16,
        ((fifo_bytes[4] as i16) << 8) | fifo_bytes[5] as i16,
        ((fifo_bytes[8] as i16) << 8) | fifo_bytes[9] as i16,
        ((fifo_bytes[12] as i16) << 8) | fifo_bytes[13] as i16,
    ];

    Quaternion {
        w: q_i[0] as f32 / 16384.0,
        x: q_i[1] as f32 / 16384.0,
        y: q_i[2] as f32 / 16384.0,
        z: q_i[3] as f32 / 16384.0,
    }
}

