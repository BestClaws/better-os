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

    /// Main update function - CRITICAL FIX for ghost touches
    async fn update(&mut self) {
        // Wait for interrupt (touch) - blocks until touch occurs
        let _ = self.pen_irq.wait_for_low().await;

        let now = Instant::now();
        if (now - self.msraw).as_millis() < MSEC_THRESHOLD {
            return;
        }

        // CRITICAL: Add debounce delay after interrupt to let hardware settle
        Timer::after(Duration::from_millis(10)).await;

        // CRITICAL: Verify touch is still active after debounce
        // If finger was lifted quickly, IRQ line goes high again
        // This prevents ghost touches from brief electrical noise
        if self.check_irq_still_low().await {
            return; // False alarm, finger already lifted
        }

        // Do complete read sequence with power management exactly like C++
        let _ = self.spi_transfer_single(0xB1).await; // Send Z1 command
        let z1 = (self.spi_transfer16(0xC1).await.unwrap_or(0) >> 3) as i32;
        let mut z = z1 + 4095;
        let z2 = (self.spi_transfer16(0x91).await.unwrap_or(0) >> 3) as i32;
        z -= z2;

        if z < 0 {
            z = 0;
        }

        let mut data = [0u16; 6];

        // Only read coordinates if pressure is sufficient
        if (z as u16) >= Z_THRESHOLD {
            let _ = self.spi_transfer16(0x91).await; // dummy X measure, 1st is always noisy
            data[0] = (self.spi_transfer16(0xD1).await.unwrap_or(0) >> 3); // Y cmd, read X
            data[1] = (self.spi_transfer16(0x91).await.unwrap_or(0) >> 3); // X cmd, read Y
            data[2] = (self.spi_transfer16(0xD1).await.unwrap_or(0) >> 3); // Y cmd, read X
            data[3] = (self.spi_transfer16(0x91).await.unwrap_or(0) >> 3); // X cmd, read Y
        }

        // Always complete the sequence with power down
        data[4] = (self.spi_transfer16(0xD0).await.unwrap_or(0) >> 3); // Last Y touch power down
        data[5] = (self.spi_transfer16(0x00).await.unwrap_or(0) >> 3); // Final read

        // CRITICAL: Power down the touch controller to reset interrupt state
        // This prevents spurious interrupts during finger lift
        let _ = self.spi_transfer_single(0x80).await; // Power down command
        Timer::after(Duration::from_millis(1)).await; // Let it settle

        // Re-enable touch detection
        let _ = self.spi_transfer_single(0xD0).await; // Enable interrupts again

        if z < 0 {
            z = 0;
        }

        if (z as u16) < Z_THRESHOLD {
            self.zraw = 0;
            return; // Don't update coordinates on insufficient pressure
        }

        self.zraw = z as u16;

        if (z as u16) >= Z_THRESHOLD {
            self.msraw = now;

            let x = Self::best_two_avg(data[0], data[2], data[4]);
            let y = Self::best_two_avg(data[1], data[3], data[5]);

            // Apply rotation
            match self.rotation {
                0 => {
                    self.xraw = 4095 - y;
                    self.yraw = x;
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

    /// Check if IRQ pin is still low (touch still active)
    /// Returns true if IRQ went high (false alarm)
    async fn check_irq_still_low(&mut self) -> bool {
        // Try to wait for high with very short timeout
        // If this succeeds quickly, it means IRQ already went high (no real touch)
        match embassy_time::with_timeout(Duration::from_millis(2), self.pen_irq.wait_for_high()).await {
            Ok(_) => true,  // IRQ went high quickly - false alarm
            Err(_) => false, // IRQ still low - real touch
        }
    }

    /// 16-bit SPI transfer - FIXED to match C++ behavior exactly
    async fn spi_transfer16(&mut self, cmd: u8) -> Result<u16, <SPI as ErrorType>::Error> {
        // C++ does: send 8-bit command, read back 16-bit data
        let tx_buf = [cmd, 0x00, 0x00];
        let mut rx_buf = [0u8; 3];

        self.spi.transfer(&mut rx_buf, &tx_buf).await?;

        // XPT2046 returns data MSB first in the next 16 bits after command
        let result = ((rx_buf[1] as u16) << 8) | (rx_buf[2] as u16);
        Ok(result)
    }

    /// Single byte transfer for commands that don't need data back
    async fn spi_transfer_single(&mut self, cmd: u8) -> Result<u8, <SPI as ErrorType>::Error> {
        let tx_buf = [cmd, 0x00]; // Send some dummy data to get response
        let mut rx_buf = [0u8; 2];

        self.spi.transfer(&mut rx_buf, &tx_buf).await?;
        Ok(rx_buf[1]) // Return the response byte
    }

    /// Best two average algorithm - exactly from C++
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
    /// Reads X, Y, Z values asynchronously - BLOCKS until touch occurs
    async fn read_xyz(&mut self) -> (u16, u16, u16) {
        self.update().await;
        (self.xraw, self.yraw, self.zraw)
    }
}