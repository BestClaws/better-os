use crate::system::hal::touch::AsyncTouch;
use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::{I2c, SevenBitAddress};
use embedded_hal::digital::OutputPin;

// FT5336 registers and masks from Zephyr code
const REG_TD_STATUS: u8 = 0x02;
const REG_P1_XH: u8 = 0x03;
const REG_G_PMODE: u8 = 0xA5;
const PMOD_HIBERNATE: u8 = 0x03;
const TOUCH_POINTS_MSK: u8 = 0x0F;
const TOUCH_ID_POS: u8 = 4;
const TOUCH_ID_MSK: u8 = 0x0F;
const TOUCH_ID_INVALID: u8 = 0x0F;
const POSITION_H_MSK: u8 = 0x0F;
const ADDR: SevenBitAddress = 0x38;
const POLL_PERIOD: Duration = Duration::from_millis(10);

// Touch point structure
#[derive(Debug, Clone, Copy)]
pub struct TSPoint {
    pub x: u16,
    pub y: u16,
}

impl TSPoint {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// Driver for the FT5336 touch controller (polling mode only)
pub struct FT5336<I2C, RST> {
    i2c: I2C,
    reset_pin: Option<RST>,
    xraw: u16,
    yraw: u16,
    pressed_old: bool,
}

impl<I2C, RST> FT5336<I2C, RST>
where
    I2C: I2c,
    RST: OutputPin,
{
    /// Create a new FT5336 driver instance with optional reset pin
    pub fn new(i2c: I2C, reset_pin: Option<RST>) -> Self {
        Self {
            i2c,
            reset_pin,
            xraw: 0,
            yraw: 0,
            pressed_old: false,
        }
    }

    /// Initialize the touch controller with hardware reset if available
    pub async fn init(&mut self) -> Result<(), ()> {
        if let Some(ref mut rst) = self.reset_pin {
            // Perform hardware reset
            rst.set_low().map_err(|_| ())?;
            Timer::after(Duration::from_millis(10)).await;
            rst.set_high().map_err(|_| ())?;
            Timer::after(Duration::from_millis(300)).await;
        }
        Ok(())
    }

    /// Get current touch point
    pub async fn get_point(&mut self) -> Option<TSPoint> {
        self.update().await;
        if self.pressed_old {
            Some(TSPoint::new(self.xraw, self.yraw))
        } else {
            None
        }
    }

    /// Update touch data
    async fn update(&mut self) {
        use embassy_time::Instant;
        let start = Instant::now();

        Timer::after(POLL_PERIOD).await;

        // Read number of touch points
        let mut buf = [0u8; 1];
        let _ = self.i2c.write_read(ADDR, &[REG_TD_STATUS], &mut buf).await;
        let points = buf[0] & TOUCH_POINTS_MSK;
        let mut pressed = false;

        if points > 0 {
            let mut coords = [0u8; 4];
            let _ = self.i2c.write_read(ADDR, &[REG_P1_XH], &mut coords).await;

            let touch_id = (coords[2] >> TOUCH_ID_POS) & TOUCH_ID_MSK;
            if touch_id != TOUCH_ID_INVALID {
                pressed = true;
                // Flip axes for this hardware: x from X regs, y from Y regs
                self.xraw = ((coords[0] & POSITION_H_MSK) as u16) << 8 | coords[1] as u16;
                self.yraw = ((coords[2] & POSITION_H_MSK) as u16) << 8 | coords[3] as u16;
            }
        }

        self.pressed_old = pressed;

        let duration = start.elapsed();
        if duration.as_millis() > 15 {
            defmt::debug!(
                "Touch update slow: {}ms, points={}, pressed={}",
                duration.as_millis(),
                points,
                pressed
            );
        }
    }

    /// Suspend the controller (hibernate mode)
    pub async fn suspend(
        &mut self,
    ) -> Result<(), <I2C as embedded_hal_async::i2c::ErrorType>::Error> {
        self.i2c.write(ADDR, &[REG_G_PMODE, PMOD_HIBERNATE]).await
    }
}

#[async_trait(?Send)]
impl<I2C, RST> AsyncTouch for FT5336<I2C, RST>
where
    I2C: I2c,
    RST: OutputPin,
{
    async fn read_xyz(&mut self) -> (u16, u16, u16) {
        self.update().await;
        (self.xraw, self.yraw, if self.pressed_old { 1 } else { 0 })
    }
}
