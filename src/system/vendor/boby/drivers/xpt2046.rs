use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer, Instant};
use embedded_hal_async::spi::{SpiDevice, ErrorType};
use embedded_hal_async::digital::Wait;
use crate::system::hal::touch::AsyncTouch;

// Thresholds and constants from the C++ code
const Z_THRESHOLD: u16 = 400;
const Z_THRESHOLD_INT: u16 = 75;
const MSEC_THRESHOLD: u64 = 3;

// Touch point structure
#[derive(Debug, Clone, Copy)]
pub struct TSPoint {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl TSPoint {
    pub fn new(x: u16, y: u16, z: u16) -> Self {
        Self { x, y, z }
    }
}

/// Driver for the XPT2046 touch controller
pub struct XPT2046<SPI, PEN> {
    spi: SPI,
    pen_irq: PEN,
    // Internal state
    xraw: u16,
    yraw: u16,
    zraw: u16,
    msraw: Instant,
    rotation: u8,
}

impl<SPI, PEN> XPT2046<SPI, PEN>
where
    SPI: SpiDevice,
    PEN: Wait,
{
    /// Create a new XPT2046 driver instance
    pub fn new(spi: SPI, pen_irq: PEN) -> Self {
        Self {
            spi,
            pen_irq,
            xraw: 0,
            yraw: 0,
            zraw: 0,
            msraw: Instant::now(),
            rotation: 1, // Default rotation
        }
    }

    /// Set the rotation (0-3)
    pub fn set_rotation(&mut self, rotation: u8) {
        self.rotation = rotation % 4;
    }

    /// Check if screen is currently being touched based on pressure threshold
    pub async fn touched(&mut self) -> bool {
        self.update().await;
        self.zraw >= Z_THRESHOLD
    }

    /// Get current touch point
    pub async fn get_point(&mut self) -> TSPoint {
        self.update().await;
        TSPoint::new(self.xraw, self.yraw, self.zraw)
    }

    /// Read raw data into provided variables
    pub async fn read_data(&mut self) -> (u16, u16, u8) {
        self.update().await;
        (self.xraw, self.yraw, (self.zraw & 0xFF) as u8)
    }

    /// Main update function - ports the core C++ update() logic
    async fn update(&mut self) {
        // Wait for interrupt (touch) - blocks until touch occurs
        let _ = self.pen_irq.wait_for_low().await;

        let now = Instant::now();
        if (now - self.msraw).as_millis() < MSEC_THRESHOLD {
            return;
        }

        // Read Z1 and Z2 for pressure calculation - following C++ sequence exactly
        let _ = self.spi_transfer(0xB1).await; // Z1 command
        let z1 = self.spi_transfer16(0xC1).await.unwrap_or(0) >> 3; // Read Z1, send Z2 command
        let mut z = (z1 as i32) + 4095;
        let z2 = self.spi_transfer16(0x91).await.unwrap_or(0) >> 3; // Read Z2, send X command
        z -= z2 as i32;

        if z < 0 {
            z = 0;
        }

        if (z as u16) < Z_THRESHOLD {
            self.zraw = 0;
            return;
        }

        self.zraw = z as u16;

        // Read coordinate data exactly like C++ - 3 measurements each for X and Y
        let mut data = [0u16; 6];

        if z >= Z_THRESHOLD as i32 {
            // Dummy X measurement (first is always noisy)
            let _ = self.spi_transfer16(0x91).await; // Dummy X, prepare for Y

            // Make 3 x-y measurements exactly like C++ code:
            data[0] = self.spi_transfer16(0xD1).await.unwrap_or(0) >> 3; // Read Y, prepare X
            data[1] = self.spi_transfer16(0x91).await.unwrap_or(0) >> 3; // Read X, prepare Y
            data[2] = self.spi_transfer16(0xD1).await.unwrap_or(0) >> 3; // Read Y, prepare X
            data[3] = self.spi_transfer16(0x91).await.unwrap_or(0) >> 3; // Read X, prepare Y
            // Last Y touch power down
            data[4] = self.spi_transfer16(0xD0).await.unwrap_or(0) >> 3; // Read Y (power down)
            data[5] = self.spi_transfer16(0x00).await.unwrap_or(0) >> 3; // Final read
        } else {
            // Set all data to 0 if pressure too low (like C++ compiler warning fix)
            data = [0; 6];
        }

        if (z as u16) >= Z_THRESHOLD {
            self.msraw = now;

            // Average pair with least distance - NOTE: C++ uses Y coords for X calc and vice versa
            let x = Self::best_two_avg(data[0], data[2], data[4]); // Y measurements become X
            let y = Self::best_two_avg(data[1], data[3], data[5]); // X measurements become Y

            // Apply rotation transformation exactly like C++
            match self.rotation {
                0 => {
                    self.xraw = 4095 - y; // Note: uses y for xraw
                    self.yraw = x;        // Note: uses x for yraw
                }
                1 => {
                    self.xraw = x;
                    self.yraw = y;
                }
                2 => {
                    self.xraw = y;
                    self.yraw = 4095 - x;
                }
                _ => { // 3
                    self.xraw = 4095 - x;
                    self.yraw = 4095 - y;
                }
            }
        }
    }

    /// SPI transfer16 function that mimics the C++ _pspi->transfer16() behavior
    async fn spi_transfer16(&mut self, cmd: u8) -> Result<u16, <SPI as ErrorType>::Error> {
        // Send command byte and read back 16-bit result
        let tx_buf = [cmd, 0x00, 0x00];
        let mut rx_buf = [0u8; 3];

        self.spi.transfer(&mut rx_buf, &tx_buf).await?;

        // The result comes in bytes 1 and 2 (byte 0 is while sending command)
        let result = ((rx_buf[1] as u16) << 8) | (rx_buf[2] as u16);
        Ok(result)
    }

    /// Single byte transfer for initial commands
    async fn spi_transfer(&mut self, cmd: u8) -> Result<u8, <SPI as ErrorType>::Error> {
        let tx_buf = [cmd];
        let mut rx_buf = [0u8; 1];

        self.spi.transfer(&mut rx_buf, &tx_buf).await?;
        Ok(rx_buf[0])
    }

    /// Port of the besttwoavg function from C++ - unchanged
    fn best_two_avg(x: u16, y: u16, z: u16) -> u16 {
        let x = x as i16;
        let y = y as i16;
        let z = z as i16;

        let da = if x > y { x - y } else { y - x };
        let db = if x > z { x - z } else { z - x };
        let dc = if z > y { z - y } else { y - z };

        if da <= db && da <= dc {
            ((x + y) >> 1) as u16
        } else if db <= da && db <= dc {
            ((x + z) >> 1) as u16
        } else {
            ((y + z) >> 1) as u16
        }
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
        self.update().await;
        (self.xraw, self.yraw, self.zraw)
    }
}