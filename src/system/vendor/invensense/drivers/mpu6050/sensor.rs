use alloc::boxed::Box;
use core::fmt::Debug;
use async_trait::async_trait;
use defmt::{error, info};
use embassy_time::{with_timeout, Duration, Timer, WithTimeout};
use embedded_hal_async::i2c::I2c;
use log::__private_api::enabled;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::system::vendor::invensense::drivers::mpu6050::dmp_firmware::DMP_FIRMWARE;

const MPU6050_CLOCK_PLL_XGYRO: u8 =  0x01;
const MPU6050_CLOCK_PLL_ZGYRO: u8 = 0x03;
const MPU6050_ADDRESS_AD0_LOW: u8  = 0x68;
const MPU6050_ADDRESS_AD0_HIGH: u8 = 0x69;
const MPU6050_DEFAULT_ADDRESS: u8 = MPU6050_ADDRESS_AD0_LOW;

const MPU6050_RA_PWR_MGMT_1: u8 = 0x6B;
const MPU6050_PWR1_CLKSEL_BIT: u8 = 2;
const MPU6050_PWR1_CLKSEL_LENGTH: u8 = 3;

const MPU6050_RA_GYRO_CONFIG: u8 = 0x1B;
const MPU6050_GCONFIG_FS_SEL_BIT: u8 = 4;
const MPU6050_GCONFIG_FS_SEL_LENGTH: u8 = 2;
const MPU6050_GYRO_FS_250: u8 = 0x00;

const MPU6050_RA_ACCEL_CONFIG: u8 = 0x1C;
const MPU6050_ACONFIG_AFS_SEL_BIT: u8 = 4;
const MPU6050_ACONFIG_AFS_SEL_LENGTH: u8 = 2;
const MPU6050_ACCEL_FS_2: u8 = 0x00;

const MPU6050_PWR1_SLEEP_BIT: u8 = 6;

const MPU6050_RA_WHO_AM_I: u8 = 0x75;
const MPU6050_WHO_AM_I_BIT: u8 = 6;
const MPU6050_WHO_AM_I_LENGTH: u8 = 6;

const MPU6050_PWR1_DEVICE_RESET_BIT: u8 = 7;

const MPU6050_RA_BANK_SEL: u8 = 0x6D;

const MPU6050_RA_MEM_START_ADDR: u8 = 0x6E;

const MPU6050_RA_MEM_R_W: u8 = 0x6F;

const MPU6050_RA_XG_OFFS_TC: u8 = 0x00;
const MPU6050_TC_OTP_BNK_VLD_BIT: u8 = 0;

const MPU6050_RA_I2C_SLV0_ADDR: u8 = 0x25;

const MPU6050_RA_USER_CTRL: u8 = 0x6A;
const MPU6050_USERCTRL_I2C_MST_EN_BIT: u8 = 5;

const MPU6050_USERCTRL_I2C_MST_RESET_BIT: u8 = 1;

const MPU6050_RA_INT_ENABLE: u8 = 0x38;

const MPU6050_INTERRUPT_FIFO_OFLOW_BIT: u8 = 4;

const MPU6050_INTERRUPT_DMP_INT_BIT: u8 = 1;

const MPU6050_RA_SMPLRT_DIV: u8 = 0x19;

const MPU6050_RA_CONFIG: u8 = 0x1A;
const MPU6050_CFG_EXT_SYNC_SET_BIT: u8 = 5;
const MPU6050_CFG_EXT_SYNC_SET_LENGTH: u8 = 3;

const MPU6050_EXT_SYNC_TEMP_OUT_L: u8 = 0x1;

const MPU6050_CFG_DLPF_CFG_BIT: u8 = 2;
const MPU6050_CFG_DLPF_CFG_LENGTH: u8 = 3;

const MPU6050_DLPF_BW_42: u8 = 0x03;



enum Error<I> where I: I2c {
    I2cError(I::Error),
    Other,
    Timeout,
}

impl<I> Debug for Error<I> where I: I2c {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::I2cError(e) => write!(f, "I2C error: {:?}", e),
            Error::Other => write!(f, "Other error"),
            Error::Timeout => write!(f, "Operation timed out"),
        }
    }
}


pub struct MPU6050<I> where I: I2c {
    i2c: I,
    address: u8,
    gyro_resolution: f64,
    accel_resolution: f64,
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {

        // set clock source to PLL with X Gyro
        self.write_bits(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, MPU6050_CLOCK_PLL_XGYRO).await;

        // set gyro resolution to 250 degrees/s
        self.gyro_resolution = 250.0 / 32768.0;
        self.write_bits(MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, MPU6050_GYRO_FS_250).await;

        // set accelerometer resolution to 2g
        self.write_bits(MPU6050_RA_ACCEL_CONFIG, MPU6050_ACONFIG_AFS_SEL_BIT, MPU6050_ACONFIG_AFS_SEL_LENGTH, MPU6050_ACCEL_FS_2).await;

        // set sleep to false
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_SLEEP_BIT, false).await;

        // get device id
        let device_id = self.read_bits(MPU6050_RA_WHO_AM_I, MPU6050_WHO_AM_I_BIT, MPU6050_WHO_AM_I_LENGTH, Duration::from_millis(1000)).await.unwrap();
        let id_match = (device_id == 0x34) || (device_id == 0xC) || (device_id == 0x3A);

        if !id_match {
            panic!("MPU6050 device ID mismatch: expected 0x34, 0xC, or 0x3A, got {:#X}", device_id);
        }

        info!("resetting mpu");
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_DEVICE_RESET_BIT, true).await;
        Timer::after_millis(30).await;

        // disable sleep mode again
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_SLEEP_BIT, false).await;


        // get MPU hardware revision
        self.set_memory_bank(0x10, true, true).await;
        self.write_byte(MPU6050_RA_MEM_START_ADDR, 0x06).await;
        info!("Checking hardware revision...");
        let revision = self.read_byte(MPU6050_RA_MEM_R_W, Duration::from_millis(1000)).await.unwrap();
        info!("Revision @ user[16][6] = {}", revision);
        info!("Resetting memory bank selection to 0...");
        self.set_memory_bank(0, true, true).await;

        info!("Reading OTP bank valid flag...");
        let is_valid = self.read_bit(MPU6050_RA_XG_OFFS_TC, MPU6050_TC_OTP_BNK_VLD_BIT, Duration::from_millis(1000)).await.unwrap();
        info!("OTP bank is: {}", is_valid);
        if (!is_valid) {
            error!("MPU6050 OTP bank is invalid");
            return;
        }
        info!("Setting slave 0 address to 0x7F...");
        self.write_byte(MPU6050_RA_I2C_SLV0_ADDR + 0, 0x7F).await;
        info!("Disabling I2C Master mode...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_EN_BIT, false).await;
        info!("Setting slave 0 address to 0x68 (self)...");
        self.write_byte(MPU6050_RA_I2C_SLV0_ADDR + 0, 0x68).await;
        info!("Resetting I2C Master control...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_RESET_BIT, true).await;
        Timer::after_millis(20).await;
        info!("Setting clock source to Z Gyro...");
        self.write_bits(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, MPU6050_CLOCK_PLL_ZGYRO).await;
        info!("Setting DMP and FIFO_OFLOW interrupts enabled...");
        self.write_byte(MPU6050_RA_INT_ENABLE, 1<<MPU6050_INTERRUPT_FIFO_OFLOW_BIT|1<<MPU6050_INTERRUPT_DMP_INT_BIT);
        info!("Setting sample rate to 200Hz...");
        self.write_byte(MPU6050_RA_SMPLRT_DIV, 4); // 1khz / (1 + 4) = 200 Hz
        info!("Setting external frame sync to TEMP_OUT_L[0]...");
        self.write_bits(MPU6050_RA_CONFIG, MPU6050_CFG_EXT_SYNC_SET_BIT, MPU6050_CFG_EXT_SYNC_SET_LENGTH, MPU6050_EXT_SYNC_TEMP_OUT_L).await.unwrap();
        info!("Setting DLPF bandwidth to 42Hz...");
        self.write_bits(MPU6050_RA_CONFIG, MPU6050_CFG_DLPF_CFG_BIT, MPU6050_CFG_DLPF_CFG_LENGTH, MPU6050_DLPF_BW_42).await.unwrap();
        info!("Setting gyro sensitivity to +/- 2000 deg/sec...");
        self.gyro_resolution = 2000.0 / 32768.0;
        self.write_bits(MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, MPU6050_GYRO_FS_2000).await;

        const MPU6050_GYRO_FS_2000: u8 = 0x03;
        const MPU6050_DMP_CODE_SIZE: u16  = 1929;
        info!("Writing DMP code to MPU memory banks ({} bytes)", MPU6050_DMP_CODE_SIZE);
        if (!writeProgMemoryBlock(DMP_FIRMWARE, MPU6050_DMP_CODE_SIZE)) {
            error!("Failed to write DMP code to MPU memory banks");
        }




    }



}


impl<I> MPU6050<I> where I: I2c {
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            address: MPU6050_DEFAULT_ADDRESS,
            gyro_resolution: 250.0 / 32768.0,
            accel_resolution: 2.0 / 32768.0,
        }
    }



    /// Set memory bank by writing to the BANK_SEL register (0x6D).
    ///
    /// # Arguments
    /// - `bank`: bank number (0–31)
    /// - `prefetch_enabled`: enables instruction prefetch if true
    /// - `user_bank`: enables user-defined bank if true
    pub async fn set_memory_bank(
        &mut self,
        bank: u8,
        prefetch_enabled: bool,
        user_bank: bool,
    ) -> Result<(), Error<I>> {
        let mut value = bank & 0x1F;
        if user_bank {
            value |= 0x20;
        }
        if prefetch_enabled {
            value |= 0x40;
        }

        self.i2c
            .write(self.address, &[MPU6050_RA_BANK_SEL, value])
            .await
            .map_err(Error::I2cError)
    }

    /// Write a single byte to the specified register.
    ///
    /// # Arguments
    /// * `reg_addr` - Register address to write to.
    /// * `data` - Byte value to write.
    pub async fn write_byte(
        &mut self,
        reg_addr: u8,
        data: u8,
    ) -> Result<(), Error<I>> {
        self.i2c
            .write(self.address, &[reg_addr, data])
            .await
            .map_err(Error::I2cError)
    }


    pub async fn write_bits(
        &mut self,
        reg_addr: u8,
        bit_start: u8,
        length: u8,
        data: u8,
    ) -> Result<(), Error<I>>
    {
        // Read the current byte from the register
        let mut buf = [0u8];
        self.i2c.write_read(self.address, &[reg_addr], &mut buf).await.map_err(Error::I2cError)?;
        let mut b = buf[0];
        // Build mask for bits to write
        // Note: bitStart in your code is the highest bit index to write, counting from 7..0
        // The mask shifts bits to cover [bitStart - length + 1 .. bitStart]
        let mask = ((1u8 << length) - 1) << (bit_start + 1 - length);
        let data_shifted = (data << (bit_start + 1 - length)) & mask;
        b &= !mask;         // Clear bits in target range
        b |= data_shifted;  // Set bits with new data
        // Write back the modified byte
        self.i2c.write(self.address, &[reg_addr, b]).await.map_err(Error::I2cError)?;
        Ok(())
    }

    pub async fn write_bit(
        &mut self,
        reg_addr: u8,
        bit_num: u8,
        data: bool,
    ) -> Result<(), Error<I>>
    {
        let mut buf = [0u8];
        // Read current register value
        self.i2c
            .write_read(self.address, &[reg_addr], &mut buf)
            .await
            .map_err(Error::I2cError)?;

        let mut b = buf[0];
        if data {
            b |= 1 << bit_num;
        } else {
            b &= !(1 << bit_num);
        }

        // Write modified byte back
        self.i2c
            .write(self.address, &[reg_addr, b])
            .await
            .map_err(Error::I2cError)?;

        Ok(())
    }

    /// Read a single byte from a register, with timeout.
    ///
    /// # Arguments
    /// * `reg_addr` - The register to read from.
    /// * `timeout` - Duration to wait before aborting read.
    ///
    /// # Returns
    /// The byte read or an error.
    pub async fn read_byte(
        &mut self,
        reg_addr: u8,
        timeout: Duration,
    ) -> Result<u8, Error<I>> {
        let mut buf = [0u8; 1];

        with_timeout(timeout, async {
            self.i2c
                .write_read(self.address, &[reg_addr], &mut buf)
                .await
        })
            .await
            .map_err(|_| Error::Timeout)?      // timeout occurred
            .map_err(Error::I2cError)?;        // I2C operation failed

        Ok(buf[0])
    }

    pub async fn read_bits(
        &mut self,
        reg_addr: u8,
        bit_start: u8,
        length: u8,
        timeout: Duration,
    ) -> Result<u8, Error<I>> {
        let mut buf = [0u8];

        // Attempt the I2C read with timeout
        with_timeout(timeout, async {
            self.i2c
                .write_read(self.address, &[reg_addr], &mut buf)
                .await
        })
            .await
            .map_err(|_| Error::Timeout)? // timeout expired
            .map_err(Error::I2cError)?;   // I2C error

        let byte = buf[0];
        let mask = ((1u8 << length) - 1) << (bit_start + 1 - length);
        let bits = (byte & mask) >> (bit_start + 1 - length);

        Ok(bits)
    }

    /// Read a single bit from an 8-bit register with a timeout.
    ///
    /// # Arguments
    /// * `reg_addr` - The register address to read from.
    /// * `bit_num` - Bit index (0..7).
    /// * `timeout` - Duration to wait before giving up.
    ///
    /// # Returns
    /// `Ok(true)` if the bit is 1, `Ok(false)` if 0, or an error.
    pub async fn read_bit(
        &mut self,
        reg_addr: u8,
        bit_num: u8,
        timeout: Duration,
    ) -> Result<bool, Error<I>> {
        let mut buf = [0u8];

        with_timeout(timeout, async {
            self.i2c
                .write_read(self.address, &[reg_addr], &mut buf)
                .await
        })
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::I2cError)?;

        let byte = buf[0];
        let bit = (byte >> bit_num) & 0x01;

        Ok(bit != 0)
    }


}