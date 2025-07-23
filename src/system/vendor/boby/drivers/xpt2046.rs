use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::{SpiDevice, ErrorType};
use embedded_hal_async::digital::Wait;
use defmt::info;
use crate::system::hal::touch::AsyncTouch;

const START_BIT: u8 = 0x80;
const X_POSITION: u8 = 0x10; // A2-A0: 001 for X-Position
const Y_POSITION: u8 = 0x50; // A2-A0: 101 for Y-Position
const Z1_POSITION: u8 = 0x30; // A2-A0: 011 for Z1
const Z2_POSITION: u8 = 0x40; // A2-A0: 100 for Z2
const MODE_12BIT: u8 = 0x00; // 12-bit resolution
const SER_DFR: u8 = 0x00;   // Differential mode for better accuracy
const POWER_DOWN: u8 = 0x00; // Power-down between conversions

pub struct XPT2046<SPI, PEN> {
    spi: SPI,
    pen_irq: PEN,
}

impl<SPI, PEN> XPT2046<SPI, PEN>
where
    SPI: SpiDevice,
    PEN: Wait,
{
    pub fn new(spi: SPI, pen_irq: PEN) -> Self {
        // // info!("Initializing XPT2046 with SPI and PENIRQ");
        XPT2046 { spi, pen_irq }
    }

    pub async fn read_xy(&mut self) -> Result<Option<(u16, u16, u16)>, <SPI as ErrorType>::Error> {
        // info!("Checking for touch interrupt (PENIRQ active low)");
        if self.pen_irq.wait_for_low().await.is_ok() {
            // info!("Touch detected, reading coordinates");
            let x = match self.read_x().await {
                Ok(x) => {
                    // info!("X coordinate read: {}", x);
                    x
                }
                Err(e) => {
                    // info!("Failed to read X coordinate due to SPI error");
                    return Err(e);
                }
            };
            let y = match self.read_y().await {
                Ok(y) => {
                    // info!("Y coordinate read: {}", y);
                    y
                }
                Err(e) => {
                    // info!("Failed to read Y coordinate due to SPI error");
                    return Err(e);
                }
            };
            let z = match self.read_z().await {
                Ok(z) => {
                    // info!("Pressure (Z) calculated: {}", z);
                    z
                }
                Err(e) => {
                    // info!("Failed to read Z pressure due to SPI error");
                    return Err(e);
                }
            };
            // info!("Touch coordinates: X = {}, Y = {}, Z = {}", x, y, z);
            Ok(Some((x, y, z)))
        } else {
            // info!("No touch detected (PENIRQ high)");
            Ok(None)
        }
    }

    async fn read_x(&mut self) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Reading X position");
        let control_byte = START_BIT | X_POSITION | MODE_12BIT | SER_DFR | POWER_DOWN;
        // info!("X control byte: 0x{:02x}", control_byte);
        let result = self.read_adc(control_byte).await?;
        Ok(result)
    }

    async fn read_y(&mut self) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Reading Y position");
        let control_byte = START_BIT | Y_POSITION | MODE_12BIT | SER_DFR | POWER_DOWN;
        // info!("Y control byte: 0x{:02x}", control_byte);
        let result = self.read_adc(control_byte).await?;
        Ok(result)
    }

    async fn read_z(&mut self) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Reading Z pressure");
        let z1 = match self.read_z1().await {
            Ok(z1) => {
                // info!("Z1 value: {}", z1);
                z1
            }
            Err(e) => {
                // info!("Failed to read Z1 due to SPI error");
                return Err(e);
            }
        };
        let z2 = match self.read_z2().await {
            Ok(z2) => {
                // info!("Z2 value: {}", z2);
                z2
            }
            Err(e) => {
                // info!("Failed to read Z2 due to SPI error");
                return Err(e);
            }
        };
        let z = z2.saturating_sub(z1);
        // info!("Calculated pressure (Z2 - Z1): {}", z);
        Ok(z)
    }

    async fn read_z1(&mut self) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Reading Z1 position");
        let control_byte = START_BIT | Z1_POSITION | MODE_12BIT | SER_DFR | POWER_DOWN;
        // info!("Z1 control byte: 0x{:02x}", control_byte);
        self.read_adc(control_byte).await
    }

    async fn read_z2(&mut self) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Reading Z2 position");
        let control_byte = START_BIT | Z2_POSITION | MODE_12BIT | SER_DFR | POWER_DOWN;
        // info!("Z2 control byte: 0x{:02x}", control_byte);
        self.read_adc(control_byte).await
    }

    async fn read_adc(&mut self, control_byte: u8) -> Result<u16, <SPI as ErrorType>::Error> {
        // info!("Starting ADC read with control byte: 0x{:02x}", control_byte);
        let mut tx_buf = [control_byte, 0x00, 0x00];
        let mut rx_buf = [0u8; 3];

        // info!("SPI transfer: sending control byte 0x{:02x}, 0x00, 0x00", control_byte);
        match self.spi.transfer(&mut rx_buf, &tx_buf).await {
            Ok(_) => {}// info!("SPI transfer successful: received {}, {}, {}", rx_buf[0], rx_buf[1], rx_buf[2]),
            Err(e) => {
                // info!("SPI transfer failed due to SPI error");
                return Err(e);
            }
        }

        // info!("Waiting for ADC acquisition (2 microseconds)");
        Timer::after(Duration::from_micros(4)).await;

        let result = ((rx_buf[1] as u16) << 4) | ((rx_buf[2] as u16) >> 4);
        // info!("Processed ADC result: {}", result);
        Ok(result)
    }

    /// Scales raw X, Y coordinates (0–4095) to display pixels (X: 0–239, Y: 0–379)
    pub fn scale_to_display(&self, raw_x: u16, raw_y: u16) -> (u16, u16) {
        let pixel_x = ((raw_x as u32 * 240) / 4096) as u16;
        let pixel_y = ((raw_y as u32 * 380) / 4096) as u16;
        info!("Scaled coordinates: raw X = {}, Y = {} to pixel X = {}, Y = {}", raw_x, raw_y, pixel_x, pixel_y);
        (pixel_x, pixel_y)
    }
}

#[async_trait(?Send)]
impl<SPI, PEN> AsyncTouch for XPT2046<SPI, PEN>
where
    SPI: SpiDevice,
    PEN: Wait,
{
    async fn read_xy(&mut self) -> (u16, u16, u16) {
        // info!("AsyncTouch read_xy called");
        match self.read_xy().await {
            Ok(Some(coords)) => {
                let (pixel_x, pixel_y) = self.scale_to_display(coords.0, coords.1);
                info!("AsyncTouch coordinates: X = {}, Y = {}, Z = {}", pixel_x, pixel_y, coords.2);
                (pixel_x, pixel_y, coords.2)
            }
            Ok(None) => {
                // info!("No touch detected in AsyncTouch read_xy, returning 0, 0, 0");
                (0, 0, 0)
            }
            Err(_) => {
                // info!("Error in AsyncTouch read_xy due to SPI error, returning 0, 0, 0");
                (0, 0, 0)
            }
        }
    }
}