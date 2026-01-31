use crate::system::hal::rtc::{AsyncRtc, RtcDateTime, RtcError};
use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::{debug, warn};
use embedded_hal_async::i2c::I2c;

const DEVICE_ADDRESS: u8 = 0x51;
const REG_CTRL1: u8 = 0x00;
const REG_SECONDS: u8 = 0x04;
const REG_MINUTES: u8 = 0x05;
const REG_HOURS: u8 = 0x06;
const REG_DAYS: u8 = 0x07;
const REG_WEEKDAY: u8 = 0x08;
const REG_MONTH: u8 = 0x09;
const REG_YEAR: u8 = 0x0A;

const CTRL1_12H_MASK: u8 = 1 << 1;
const CTRL1_STOP_MASK: u8 = 1 << 5;
const CLOCK_INTEGRITY_FLAG: u8 = 1 << 7;

pub struct Pcf85063<I2C> {
    i2c: I2C,
}

impl<I2C> Pcf85063<I2C>
where
    I2C: I2c,
{
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    async fn read_register(&mut self, register: u8) -> Result<u8, RtcError> {
        let mut buf = [0u8; 1];
        self.read_registers(register, &mut buf).await?;
        Ok(buf[0])
    }

    async fn write_register(&mut self, register: u8, value: u8) -> Result<(), RtcError> {
        let buf = [register, value];
        self.i2c
            .write(DEVICE_ADDRESS, &buf)
            .await
            .map_err(|_| RtcError::Bus)
    }

    async fn read_registers(&mut self, start: u8, buffer: &mut [u8]) -> Result<(), RtcError> {
        self.i2c
            .write_read(DEVICE_ADDRESS, &[start], buffer)
            .await
            .map_err(|_| RtcError::Bus)
    }

    async fn write_registers(&mut self, start: u8, data: &[u8]) -> Result<(), RtcError> {
        let mut buf = [0u8; 8];
        if data.len() + 1 > buf.len() {
            return Err(RtcError::InvalidData);
        }
        buf[0] = start;
        buf[1..=data.len()].copy_from_slice(data);
        self.i2c
            .write(DEVICE_ADDRESS, &buf[..=data.len()])
            .await
            .map_err(|_| RtcError::Bus)
    }
}

fn bcd_to_bin(value: u8) -> u8 {
    ((value >> 4) * 10) + (value & 0x0F)
}

fn bin_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

#[async_trait(?Send)]
impl<I2C> AsyncRtc for Pcf85063<I2C>
where
    I2C: I2c + 'static,
{
    async fn init(&mut self) -> Result<(), RtcError> {
        // Ensure 24-hour mode and that the oscillator is running.
        let mut ctrl1 = self.read_register(REG_CTRL1).await?;
        if ctrl1 & CTRL1_12H_MASK != 0 {
            ctrl1 &= !CTRL1_12H_MASK;
        }
        if ctrl1 & CTRL1_STOP_MASK != 0 {
            debug!("PCF85063 oscillator stopped, attempting restart");
            ctrl1 &= !CTRL1_STOP_MASK;
        }
        self.write_register(REG_CTRL1, ctrl1).await?;
        Ok(())
    }

    async fn now(&mut self) -> Result<RtcDateTime, RtcError> {
        let mut buf = [0u8; 7];
        self.read_registers(REG_SECONDS, &mut buf).await?;

        if buf[0] & CLOCK_INTEGRITY_FLAG != 0 {
            warn!("PCF85063 clock integrity lost, time invalid");
            return Err(RtcError::InvalidData);
        }

        let second = bcd_to_bin(buf[0] & 0x7F);
        let minute = bcd_to_bin(buf[1] & 0x7F);
        let hour = bcd_to_bin(buf[2] & 0x3F);
        let day = bcd_to_bin(buf[3] & 0x3F);
        let weekday = buf[4] & 0x07;
        let month = bcd_to_bin(buf[5] & 0x1F);
        let year = 2000 + bcd_to_bin(buf[6]) as u16;

        Ok(RtcDateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
            weekday,
        })
    }

    async fn set(&mut self, datetime: &RtcDateTime) -> Result<(), RtcError> {
        let data = [
            bin_to_bcd(datetime.second) & 0x7F,
            bin_to_bcd(datetime.minute) & 0x7F,
            bin_to_bcd(datetime.hour) & 0x3F,
            bin_to_bcd(datetime.day) & 0x3F,
            datetime.weekday & 0x07,
            bin_to_bcd(datetime.month) & 0x1F,
            bin_to_bcd((datetime.year % 100) as u8),
        ];

        self.write_registers(REG_SECONDS, &data).await?;
        Ok(())
    }
}
