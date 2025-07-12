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
use crate::util::math::primitives::Quaternion;
use micromath::F32Ext;

const DMP_PACKET_SIZE: u16 = 42;

const MPU6050_DMP_FIFO_RATE_DIVISOR: u8 = 0x01;
const MPU6050_RA_DMP_CFG_1: u8 = 0x70;
const MPU6050_RA_DMP_CFG_2: u8 = 0x71;
const MPU6050_RA_MOT_THR: u8 = 0x1F;
const MPU6050_RA_ZRMOT_THR: u8 = 0x21;

const MPU6050_RA_MOT_DUR: u8 = 0x20;
const MPU6050_RA_ZRMOT_DUR: u8 = 0x22;

const MPU6050_USERCTRL_FIFO_EN_BIT: u8 = 6;

const MPU6050_USERCTRL_DMP_RESET_BIT: u8 = 3;

const MPU6050_USERCTRL_DMP_EN_BIT: u8 = 7;

const MPU6050_USERCTRL_FIFO_RESET_BIT: u8 = 2;
const MPU6050_RA_INT_STATUS: u8 = 0x3A;

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

const MPU6050_GYRO_FS_2000: u8 = 0x03;
const MPU6050_DMP_MEMORY_CHUNK_SIZE: usize = 16;
const MPU6050_DMP_CODE_SIZE: u16  = 1929;

pub enum Error<I> where I: I2c {
    I2cError(I::Error),
    Other,
    Timeout,
    FirmwareUploadVerificationFailed,
    BufferTooSmall,
    BufferTooLarge,
}

impl<I> Debug for Error<I> where I: I2c {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::I2cError(e) => write!(f, "I2C error: {:?}", e),
            Error::Other => write!(f, "Other error"),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::FirmwareUploadVerificationFailed => write!(f, "Firmware upload verification failed"),
            Error::BufferTooSmall => write!(f, "Buffer too small for operation"),
            Error::BufferTooLarge => write!(f, "Buffer too large for operation"),
        }
    }
}


pub struct MPU6050<I> where I: I2c {
    i2c: I,
    address: u8,
    gyro_resolution: f64,
    accel_resolution: f64,
    ready: bool
}

#[async_trait(?Send)]
impl<I> AsyncGyroAccelerometer for MPU6050<I>  where I: I2c {

    async fn init(&mut self) {

        // set clock source to PLL with X Gyro
        self.write_bits(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, MPU6050_CLOCK_PLL_XGYRO).await.unwrap();

        // set gyro resolution to 250 degrees/s
        self.gyro_resolution = 250.0 / 32768.0;
        self.write_bits(MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, MPU6050_GYRO_FS_250).await.unwrap();

        // set accelerometer resolution to 2g
        self.write_bits(MPU6050_RA_ACCEL_CONFIG, MPU6050_ACONFIG_AFS_SEL_BIT, MPU6050_ACONFIG_AFS_SEL_LENGTH, MPU6050_ACCEL_FS_2).await.unwrap();

        // set sleep to false
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_SLEEP_BIT, false).await.unwrap();

        // get device id
        let device_id = self.get_device_id().await.unwrap();
        let id_match = (device_id == 0x34) || (device_id == 0xC) || (device_id == 0x3A);

        if !id_match {
            panic!("MPU6050 device ID mismatch: expected 0x34, 0xC, or 0x3A, got {:#X}", device_id);
        }

        info!("resetting mpu");
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_DEVICE_RESET_BIT, true).await.unwrap();
        Timer::after_millis(30).await;

        // disable sleep mode again
        self.write_bit(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_SLEEP_BIT, false).await.unwrap();


        // get MPU hardware revision
        self.set_memory_bank(0x10, true, true).await.unwrap();
        self.write_byte(MPU6050_RA_MEM_START_ADDR, 0x06).await.unwrap();
        info!("Checking hardware revision...");
        let revision = self.read_byte(MPU6050_RA_MEM_R_W, Duration::from_millis(1000)).await.unwrap();
        info!("Revision @ user[16][6] = {}", revision);
        info!("Resetting memory bank selection to 0...");
        self.set_memory_bank(0, true, true).await.unwrap();

        info!("Reading OTP bank valid flag...");
        let is_valid = self.read_bit(MPU6050_RA_XG_OFFS_TC, MPU6050_TC_OTP_BNK_VLD_BIT, Duration::from_millis(1000)).await.unwrap();
        info!("OTP bank is: {}", is_valid);
        if (!is_valid) {
            error!("MPU6050 OTP bank is invalid");
            return;
        }
        info!("Setting slave 0 address to 0x7F...");
        self.write_byte(MPU6050_RA_I2C_SLV0_ADDR, 0x7F).await.unwrap();
        info!("Disabling I2C Master mode...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_EN_BIT, false).await.unwrap();
        info!("Setting slave 0 address to 0x68 (self)...");
        self.write_byte(MPU6050_RA_I2C_SLV0_ADDR, 0x68).await.unwrap();
        info!("Resetting I2C Master control...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_I2C_MST_RESET_BIT, true).await.unwrap();
        Timer::after_millis(20).await;
        info!("Setting clock source to Z Gyro...");
        self.write_bits(MPU6050_RA_PWR_MGMT_1, MPU6050_PWR1_CLKSEL_BIT, MPU6050_PWR1_CLKSEL_LENGTH, MPU6050_CLOCK_PLL_ZGYRO).await.unwrap();
        info!("Setting DMP and FIFO_OFLOW interrupts enabled...");
        // self.write_byte(MPU6050_RA_INT_ENABLE, (1 << MPU6050_INTERRUPT_FIFO_OFLOW_BIT) | (1 << MPU6050_INTERRUPT_DMP_INT_BIT)).await.unwrap();
        self.write_byte(MPU6050_RA_INT_ENABLE,  (1 << MPU6050_INTERRUPT_DMP_INT_BIT)).await.unwrap();
        info!("Setting sample rate to 200Hz...");
        self.write_byte(MPU6050_RA_SMPLRT_DIV, 4).await.unwrap(); // 1khz / (1 + 4) = 200 Hz
        info!("Setting external frame sync to TEMP_OUT_L[0]...");
        self.write_bits(MPU6050_RA_CONFIG, MPU6050_CFG_EXT_SYNC_SET_BIT, MPU6050_CFG_EXT_SYNC_SET_LENGTH, MPU6050_EXT_SYNC_TEMP_OUT_L).await.unwrap();
        info!("Setting DLPF bandwidth to 42Hz...");
        self.write_bits(MPU6050_RA_CONFIG, MPU6050_CFG_DLPF_CFG_BIT, MPU6050_CFG_DLPF_CFG_LENGTH, MPU6050_DLPF_BW_42).await.unwrap();
        info!("Setting gyro sensitivity to +/- 2000 deg/sec...");
        self.gyro_resolution = 2000.0 / 32768.0;
        self.write_bits(MPU6050_RA_GYRO_CONFIG, MPU6050_GCONFIG_FS_SEL_BIT, MPU6050_GCONFIG_FS_SEL_LENGTH, MPU6050_GYRO_FS_2000).await.unwrap();


        info!("Writing DMP code to MPU memory banks ({} bytes)", MPU6050_DMP_CODE_SIZE);
        let Ok(_) = self.write_prog_memory_block(DMP_FIRMWARE, 0, 0, true).await else  {
            error!("Failed to write DMP code to MPU memory banks");
            return;
        };

        info!("firmware upload successful");




        const DMP_UPDATE: [u8; 2] = [0x00, MPU6050_DMP_FIFO_RATE_DIVISOR];
        self.write_memory_block(&DMP_UPDATE, 0x02, 0x16, false).await.unwrap();

        //write start address MSB into register
        self.write_byte(MPU6050_RA_DMP_CFG_1, 0x03).await.unwrap();
        //write start address LSB into register
        self.write_byte(MPU6050_RA_DMP_CFG_2, 0x00).await.unwrap();

        info!("clearing top bank");
        self.write_bit(MPU6050_RA_XG_OFFS_TC, MPU6050_TC_OTP_BNK_VLD_BIT, false).await.unwrap();
        info!("Setting motion detection threshold to 2...");
        self.write_byte(MPU6050_RA_MOT_THR, 2).await.unwrap();
        info!("settings zero motion detection threshold to 156...");
        self.write_byte(MPU6050_RA_ZRMOT_THR, 156).await.unwrap();
        info!("Setting motion detection duration to 80...");
        self.write_byte(MPU6050_RA_MOT_DUR, 80).await.unwrap();
        info!("Setting zero-motion detection duration to 0...");
        self.write_byte(MPU6050_RA_ZRMOT_DUR, 0).await.unwrap();
        info!("Enabling FIFO...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_FIFO_EN_BIT, true).await.unwrap();
        info!("Resetting DMP...");
        self.reset_dmp().await.unwrap();

        info!("DMP is good to go! Finally.");
        info!("Disabling DMP (you turn it on later)...");
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_DMP_EN_BIT, false).await.unwrap();

        info!("hard defaulting to  42-byte  DMP packet buffer...");


        info!("Resetting FIFO and clearing INT status one last time...");
        self.reset_fifo().await;
        // reading clears the bits.
        let _ = self.read_byte(MPU6050_RA_INT_STATUS, Duration::from_millis(1000)).await.unwrap();


        info!("DPM INITIALIZED. EVERYBODY CLAP!");

        // MY LOGIC START
        // enable dmp
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_DMP_EN_BIT, true).await.unwrap();


        // self.calibrate_accel(6).await.unwrap();
        // self.calibrate_gyro(6).await.unwrap();





    }

    async fn reset_fifo(&mut self)  {
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_FIFO_RESET_BIT, true).await.unwrap();
    }


    async fn get_accelerometer_data(&mut self) -> (f32, f32, f32) {
        (1., 1. , 1.)
    }

     async fn get_temperature_celsius(
        &mut self,
    ) -> u8 {
        const MPU6050_RA_TEMP_OUT_H: u8 = 0x41;
        let mut buf = [0u8; 2];

        self.read_bytes(MPU6050_RA_TEMP_OUT_H, &mut buf, Duration::from_millis(1000)).await.unwrap();

        let raw_temp: i16 = ((buf[0] as i16) << 8) | buf[1] as i16;

        // Convert to Celsius using datasheet formula
        let temp_c = (raw_temp as f32 / 340.0) + 36.53;

        temp_c as u8
    }
    async fn get_orientation(&mut self) -> Quaternion {
        loop {
            let mut fifo_count = self.get_fifo_count().await;
            if fifo_count < DMP_PACKET_SIZE {
                debug!("waiting more for dmp packet");
                Timer::after_millis(50).await; // sampling rate is 20 hz
                continue;
            }

            let mut packet: [u8; DMP_PACKET_SIZE as usize] = [0; DMP_PACKET_SIZE as usize];
            // Drain old/stale packets
            while fifo_count > DMP_PACKET_SIZE {
                packet = self.get_fifo_bytes().await;
                fifo_count -= DMP_PACKET_SIZE;
            }
            debug!("packet: {}", packet);
            return self.dmp_get_quaternion(&packet);




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
            ready: false,
        }
    }





    pub fn dmp_get_quaternion(&self, packet: &[u8; 42]) -> Quaternion {
        let q_i16 = [
            i16::from_be_bytes([packet[0],  packet[1]]),
            i16::from_be_bytes([packet[4],  packet[5]]),
            i16::from_be_bytes([packet[8],  packet[9]]),
            i16::from_be_bytes([packet[12], packet[13]]),
        ];

        Quaternion {
            w: q_i16[0] as f32 / 16384.0,
            x: q_i16[1] as f32 / 16384.0,
            y: q_i16[2] as f32 / 16384.0,
            z: q_i16[3] as f32 / 16384.0,
        }
    }


    async fn get_fifo_count(&mut self) -> u16 {
        const MPU6050_RA_FIFO_COUNTH: u8 = 0x72;
        let mut fbuf: [u8; 2] = [0, 0];
        loop {
            let Ok(_) = self.read_bytes(MPU6050_RA_FIFO_COUNTH, &mut fbuf, Duration::from_millis(1000)).await else {
                info!("fifo count read wait");
                Timer::after_millis(100).await;
                continue;
            };
            let total = ((fbuf[0] as u16) << 8) | (fbuf[1] as u16);
            return total


        }





    }

    async fn get_fifo_bytes(&mut self) -> [u8; 42] {
        const MPU6050_RA_FIFO_R_W: u8 = 0x74;
        let mut fbuf: [u8; 42]  = [0; 42];

        loop {
            let Ok(_) = self.read_bytes(MPU6050_RA_FIFO_R_W, &mut fbuf, Duration::from_millis(1000)).await else {
                Timer::after_millis(100).await;
                continue;
            };

            return fbuf;
        }

    }


    pub async fn reset_dmp(&mut self) -> Result<(), Error<I>> {
        self.write_bit(MPU6050_RA_USER_CTRL, MPU6050_USERCTRL_DMP_RESET_BIT, true).await
    }


    pub async fn get_device_id(&mut self) -> Result<u8, Error<I>> {
        self.read_bits(MPU6050_RA_WHO_AM_I, MPU6050_WHO_AM_I_BIT, MPU6050_WHO_AM_I_LENGTH, Duration::from_millis(1000)).await
    }

    pub async fn get_full_scale_accel_range(&mut self) -> Result<u8, Error<I>> {
        self.read_bits(MPU6050_RA_ACCEL_CONFIG, MPU6050_ACONFIG_AFS_SEL_BIT, MPU6050_ACONFIG_AFS_SEL_LENGTH, Duration::from_millis(1000)).await
    }


    pub async fn calibrate_accel(
        &mut self,
        loops: u8,
    ) -> Result<(), Error<I>> {
        let mut kp = 0.3f32;
        let mut ki = 20.0f32;
        let scale = (100.0 - self.map(loops as i32, 1, 5, 20, 0) as f32) * 0.01;
        kp *= scale;
        ki *= scale;

        self.pid(0x3B, kp, ki, loops).await
    }

    pub async fn calibrate_gyro(&mut self, loops: u8) -> Result<(), Error<I>> {
        let mut kp = 0.3f32;
        let mut ki = 90.0f32;

        let scale = (100.0 - self.map(loops as i32, 1, 5, 20, 0) as f32)  * 0.01;
        kp *= scale;
        ki *= scale;

        self.pid(0x43, kp, ki, loops).await
    }


    // Helper: emulate Arduino’s map() function
    fn map(&self, x: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
        (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
    }

    pub async fn pid(
        &mut self,
        read_addr: u8,
        mut kp: f32,
        mut ki: f32,
        loops: u8,
    ) -> Result<(), Error<I>> {
        use embassy_time::Timer;

        let save_addr = if read_addr == 0x3B {

            let dev_id = self.get_device_id().await?;
            info!("Device ID: {:#X}", dev_id);
            if dev_id < 0x38 {
                0x06
            } else {
                0x77
            }
        } else {
            0x13
        };

        let shift = if save_addr == 0x77 { 3 } else { 2 };

        let mut bit_zero = [0i16; 3];
        let mut iterm = [0f32; 3];
        let mut reading;
        let mut data;

        let gravity = if read_addr == 0x3B {
            32768 >> self.get_full_scale_accel_range().await?
        } else {
            8192
        };

        // Initial ITerm and bitZero read
        for i in 0..3usize {
            let mut word = [0u16; 1];
            self.read_words(save_addr + (i * shift)  as u8, &mut word, Duration::from_millis(1000)).await?;
            data = word[0] as i16;
            reading = data as f32;

            if save_addr != 0x13 {
                bit_zero[i] = data & 1;
                iterm[i] = reading * 8.0;
            } else {
                iterm[i] = reading * 4.0;
            }
        }

        for _ in 0..loops {
            let mut esample = 0;

            for c in 0..100 {
                let mut esum = 0;

                for i in 0..3usize {
                    let mut word = [0u16; 1];
                    self.read_words(read_addr + (i * 2) as u8, &mut word, Duration::from_millis(1000)).await?;
                    data = word[0] as i16;
                    reading = data as f32;

                    if read_addr == 0x3B && i == 2 {
                        reading -= gravity as f32;
                    }

                    let error = -reading;
                    esum += reading.abs() as u32;

                    let pterm = kp * error;
                    iterm[i] += (error * 0.001) * ki;

                    let mut output = if save_addr != 0x13 {
                        let mut val = ((pterm + iterm[i]) / 8.0).round() as i16;
                        val = (val & !1) | bit_zero[i]; // Preserve bit 0
                        val
                    } else {
                        ((pterm + iterm[i]) / 4.0).round() as i16
                    };

                    let word: [u16; 1] = [output as u16];
                    self.write_words(save_addr + (i * shift) as u8, &word, Duration::from_millis(1000)).await?;
                }

                if c == 99 && esum > 1000 {
                    // retry loop
                    continue;
                }

                if (esum as f32 * if read_addr == 0x3B { 0.05 } else { 1.0 }) < 5.0 {
                    esample += 1;
                }

                if esum < 100 && c > 10 && esample >= 10 {
                    break;
                }

                Timer::after_millis(1).await;
            }

            kp *= 0.75;
            ki *= 0.75;

            for i in 0..3 {
                let mut output = if save_addr != 0x13 {
                    let mut val = (iterm[i] / 8.0).round() as i16;
                    val = (val & !1) | bit_zero[i]; // Preserve bit 0
                    val
                } else {
                    (iterm[i] / 4.0).round() as i16
                };

                let word: [u16; 1] = [output as u16];
                self.write_words(save_addr + (i * shift) as u8, &word, Duration::from_millis(1000)).await?;
            }
        }

        self.reset_fifo().await;
        self.reset_dmp().await?;

        Ok(())
    }




    pub async fn set_memory_start_address(
        &mut self,
        address: u8,
    ) -> Result<(), Error<I>> {
        self.write_byte(MPU6050_RA_MEM_START_ADDR, address).await
    }

    pub async fn write_prog_memory_block(
        &mut self,
        data: &[u8],
        bank: u8,
        address: u8,
        verify: bool,
    ) -> Result<(), Error<I>> {
        self.write_memory_block(data, bank, address, verify).await
    }

    pub async fn write_memory_block(
        &mut self,
        data: &[u8],
        mut bank: u8,
        mut address: u8,
        verify: bool,
    ) -> Result<(), Error<I>> {
        let mut i = 0;
        let data_size = data.len();

        let mut verify_buf = [0u8; MPU6050_DMP_MEMORY_CHUNK_SIZE];

        while i < data_size {
            let mut chunk_size = MPU6050_DMP_MEMORY_CHUNK_SIZE;

            if i + chunk_size > data_size {
                chunk_size = data_size - i;
            }

            if chunk_size > (256 - address as usize) {
                chunk_size = 256 - address as usize;
            }

            let chunk = &data[i..i + chunk_size];

            self.set_memory_bank(bank, false, false).await?;
            self.set_memory_start_address(address).await?;



            let mut write_buf = [0u8; MPU6050_DMP_MEMORY_CHUNK_SIZE + 1];
            write_buf[0] = MPU6050_RA_MEM_R_W;
            write_buf[1..=chunk_size].copy_from_slice(&chunk[..chunk_size]);

            self.i2c
                .write(self.address, &write_buf[..=chunk_size])
                .await
                .map_err(Error::I2cError)?;

            if verify {
                self.set_memory_bank(bank, false, false).await?;
                self.set_memory_start_address(address).await?;

                self.i2c
                    .write_read(self.address, &[MPU6050_RA_MEM_R_W], &mut verify_buf[..chunk_size])
                    .await
                    .map_err(Error::I2cError)?;

                if &verify_buf[..chunk_size] != chunk {
                    return Err(Error::FirmwareUploadVerificationFailed);
                }
            }

            i += chunk_size;
            address = address.wrapping_add(chunk_size as u8);
            if i < data_size && address == 0 {
                bank = bank.wrapping_add(1);
            }
        }

        Ok(())
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


    pub async fn read_words(
        &mut self,
        reg_addr: u8,
        buffer: &mut [u16],
        timeout: Duration,
    ) -> Result<usize, Error<I>> {
        let mut raw_bytes = [0u8; 64]; // supports up to 32 words
        let byte_len = buffer.len() * 2;
        if byte_len > raw_bytes.len() {
            return Err(Error::BufferTooSmall);
        }

        // Perform I2C read with timeout
        let result = with_timeout(timeout, async {
            self.i2c.write_read(self.address, &[reg_addr], &mut raw_bytes[..byte_len]).await
        }).await;

        match result {
            Ok(Ok(())) => {
                for i in 0..buffer.len() {
                    let msb = raw_bytes[2 * i];
                    let lsb = raw_bytes[2 * i + 1];
                    buffer[i] = ((msb as u16) << 8) | (lsb as u16);
                }
                Ok(buffer.len())
            }
            Ok(Err(e)) => Err(Error::I2cError(e)),
            Err(_) => Err(Error::Timeout),
        }
    }

    pub async fn write_words(
        &mut self,
        reg_addr: u8,
        data: &[u16],
        timeout: Duration,
    ) -> Result<(), Error<I>> {
        // Limit: 32 words = 64 bytes
        if data.len() > 32 {
            return Err(Error::BufferTooLarge);
        }

        // Convert each u16 to big-endian (MSB first)
        let mut buffer = [0u8; 65]; // max: 1 byte reg + 64 bytes data
        buffer[0] = reg_addr;

        for (i, word) in data.iter().enumerate() {
            buffer[1 + 2 * i] = (*word >> 8) as u8;     // MSB
            buffer[1 + 2 * i + 1] = (*word & 0xFF) as u8; // LSB
        }

        // Total bytes: 1 + (2 * number of words)
        let total_len = 1 + data.len() * 2;

        // I2C write with timeout
        let result = with_timeout(timeout, async {
            self.i2c.write(self.address, &buffer[..total_len]).await
        })
            .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(Error::I2cError(e)),
            Err(_) => Err(Error::Timeout),
        }
    }



    /// l1: seems ok.
    /// Reads multiple bytes from a register with an optional timeout.
    pub async fn read_bytes(
        &mut self,
        reg: u8,
        buffer: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, Error<I>>

    {

        // Read into the buffer with timeout
        let res = with_timeout(timeout, async {
            self.i2c.write_read(self.address, &[reg], buffer).await
        })
            .await;

        match res {
            Ok(Ok(())) => Ok(buffer.len()),
            Ok(Err(e)) => Err(Error::I2cError(e)),
            Err(_) => Err(Error::Timeout),
        }
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