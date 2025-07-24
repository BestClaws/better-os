use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::{SpiDevice, ErrorType};
use embedded_hal_async::digital::Wait;
use crate::system::hal::touch::AsyncTouch;

/// Command bit masks for XPT2046 touch controller
const START_BIT: u8 = 0x80;
const MODE_12BIT: u8 = 0x00;     // 12-bit mode
const SER_DFR: u8 = 0x00;        // Differential reference mode
const POWER_DOWN: u8 = 0x00;     // Power down between conversions

const X_POSITION: u8 = 0x10;     // A2-A0 = 001
const Y_POSITION: u8 = 0x50;     // A2-A0 = 101
const Z1_POSITION: u8 = 0x30;    // A2-A0 = 011
const Z2_POSITION: u8 = 0x40;    // A2-A0 = 100

/// Driver for the XPT2046 touch controller
pub struct XPT2046<SPI, PEN> {
    spi: SPI,
    pen_irq: PEN,
}

impl<SPI, PEN> XPT2046<SPI, PEN>
where
    SPI: SpiDevice,
    PEN: Wait,
{
    /// Create a new XPT2046 driver instance
    pub fn new(spi: SPI, pen_irq: PEN) -> Self {
        Self { spi, pen_irq }
    }

    /// Reads raw ADC values for X, Y, and Z (pressure)
    pub async fn read_raw_xyz(&mut self) -> Result<Option<(u16, u16, u16)>, <SPI as ErrorType>::Error> {
        if self.pen_irq.wait_for_low().await.is_ok() {
            let x = self.read_adc_channel(X_POSITION).await?;
            let y = self.read_adc_channel(Y_POSITION).await?;
            let z1 = self.read_adc_channel(Z1_POSITION).await?;
            let z2 = self.read_adc_channel(Z2_POSITION).await?;
            let z = z2.saturating_sub(z1);
            Ok(Some((x, y, z)))
        } else {
            Ok(None)
        }
    }

    /// Internal helper: reads ADC value from a given channel
    async fn read_adc_channel(&mut self, command: u8) -> Result<u16, <SPI as ErrorType>::Error> {
        let control_byte = START_BIT | command | MODE_12BIT | SER_DFR | POWER_DOWN;
        let tx_buf = [control_byte, 0x00, 0x00];
        let mut rx_buf = [0u8; 3];

        self.spi.transfer(&mut rx_buf, &tx_buf).await?;
        Timer::after(Duration::from_micros(4)).await;

        let result = ((rx_buf[1] as u16) << 4) | ((rx_buf[2] as u16) >> 4);
        Ok(result)
    }
}



#[async_trait(?Send)]
impl<SPI, PEN> AsyncTouch for XPT2046<SPI, PEN>
where
    SPI: SpiDevice,
    PEN: Wait,
{
    /// Reads X, Y, Z values asynchronously, returns (0, 0, 0) on error or no touch
    async fn read_xyz(&mut self) -> (u16, u16, u16) {
        match self.read_raw_xyz().await {
            Ok(Some((x, y, z))) => (x, y, z),
            _ => (0, 0, 0),
        }
    }
}
