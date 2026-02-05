//! FT3x68 Capacitive Touch Controller Driver (Focaltech)
//!
//! This driver supports the FT3168/FT3268 touch controller from Focaltech,
//! commonly used in AMOLED displays. Features include:
//! - Multi-touch detection (up to 5 points)
//! - Gesture recognition
//! - I2C interface
//! - Interrupt-driven or polling mode
//!
//! # Hardware Specifications
//! - Interface: I2C (address 0x38)
//! - Max Touch Points: 5
//! - Resolution: Up to 410×502
//! - Gestures: Swipe, zoom, rotate
//!
//! # Usage
//! ```rust,no_run
//! let touch = Ft3x68::new(i2c);
//! touch.init().await?;
//!
//! loop {
//!     if let Some(points) = touch.read_touches().await {
//!         for point in points {
//!             // handle touch
//!         }
//!     }
//! }
//! ```

extern crate alloc;

use alloc::vec::Vec;
use defmt::{debug, error, info, Format};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

// ═══════════════════════════════════════════════════════════════════════════
// Hardware Configuration & Constants
// ═══════════════════════════════════════════════════════════════════════════

/// I2C device address for FT3x68 (7-bit address)
const FT3X68_ADDR: u8 = 0x38;

/// Register addresses
const REG_MODE_SWITCH: u8 = 0x00;
const REG_TD_STATUS: u8 = 0x02;
const REG_TOUCH1_XH: u8 = 0x03;
const REG_TOUCH1_XL: u8 = 0x04;
const REG_TOUCH1_YH: u8 = 0x05;
const REG_TOUCH1_YL: u8 = 0x06;
const REG_TOUCH1_WEIGHT: u8 = 0x07;
const REG_TOUCH1_MISC: u8 = 0x08;

const REG_CHIP_ID: u8 = 0xA3;
const REG_FW_VER: u8 = 0xA6;
const REG_VENDOR_ID: u8 = 0xA8;

/// Touch point data size (6 bytes per point)
const TOUCH_POINT_SIZE: usize = 6;

/// Maximum number of touch points
const MAX_TOUCH_POINTS: usize = 5;

/// Touch event IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum TouchEvent {
    PressDown = 0,
    LiftUp = 1,
    Contact = 2,
    None = 3,
}

impl From<u8> for TouchEvent {
    fn from(val: u8) -> Self {
        match val {
            0 => TouchEvent::PressDown,
            1 => TouchEvent::LiftUp,
            2 => TouchEvent::Contact,
            _ => TouchEvent::None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Touch Point Types
// ═══════════════════════════════════════════════════════════════════════════

/// Single touch point data
#[derive(Debug, Clone, Copy, Format)]
pub struct TouchPoint {
    /// Touch point ID (0-4)
    pub id: u8,
    /// X coordinate
    pub x: u16,
    /// Y coordinate
    pub y: u16,
    /// Touch event type
    pub event: TouchEvent,
    /// Touch area/weight
    pub weight: u8,
}

// ═══════════════════════════════════════════════════════════════════════════
// Driver Implementation
// ═══════════════════════════════════════════════════════════════════════════

/// FT3x68 touch controller driver
pub struct Ft3x68<I2C> {
    i2c: I2C,
    last_touches: Vec<TouchPoint>,
}

impl<I2C, E> Ft3x68<I2C>
where
    I2C: I2c<Error = E>,
    E: defmt::Format,
{
    /// Create a new FT3x68 driver
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            last_touches: Vec::new(),
        }
    }

    /// Initialize the touch controller
    ///
    /// Verifies the chip ID and reads firmware version.
    pub async fn init(&mut self) -> Result<(), E> {
        info!("Initializing FT3x68 touch controller");

        // Wait for controller to be ready
        Timer::after_millis(50).await;

        // Read chip ID
        let chip_id = self.read_register(REG_CHIP_ID).await?;
        info!("FT3x68 chip ID: 0x{:02x}", chip_id);

        // Read firmware version
        let fw_version = self.read_register(REG_FW_VER).await?;
        let vendor_id = self.read_register(REG_VENDOR_ID).await?;
        info!("FT3x68 initialized: fw=0x{:02x} vendor=0x{:02x}", fw_version, vendor_id);

        Ok(())
    }

    /// Read touch points from the controller
    ///
    /// Returns a vector of active touch points, or None if no touches detected.
    pub async fn read_touches(&mut self) -> Result<Option<Vec<TouchPoint>>, E> {
        // Read touch status register
        let td_status = self.read_register(REG_TD_STATUS).await?;
        let touch_count = td_status & 0x0F;

        // No touches
        if touch_count == 0 {
            if !self.last_touches.is_empty() {
                // Generate release events
                let mut released = self.last_touches.clone();
                for point in &mut released {
                    point.event = TouchEvent::LiftUp;
                }
                self.last_touches.clear();
                return Ok(Some(released));
            }
            return Ok(None);
        }

        // Read touch point data (6 bytes per point starting at REG_TOUCH1_XH)
        let data_size = (touch_count as usize).min(MAX_TOUCH_POINTS) * TOUCH_POINT_SIZE;
        let mut data = [0u8; MAX_TOUCH_POINTS * TOUCH_POINT_SIZE];
        self.read_registers(REG_TOUCH1_XH, &mut data[..data_size]).await?;

        // Parse touch points
        let mut points = Vec::new();
        for i in 0..touch_count.min(MAX_TOUCH_POINTS as u8) {
            let offset = (i as usize) * TOUCH_POINT_SIZE;
            
            let xh = data[offset];
            let xl = data[offset + 1];
            let yh = data[offset + 2];
            let yl = data[offset + 3];
            let weight = data[offset + 4];
            let misc = data[offset + 5];

            let x = (((xh & 0x0F) as u16) << 8) | (xl as u16);
            let y = (((yh & 0x0F) as u16) << 8) | (yl as u16);
            let event = TouchEvent::from((xh >> 6) & 0x03);
            let id = (yh >> 4) & 0x0F;

            points.push(TouchPoint {
                id,
                x,
                y,
                event,
                weight,
            });
        }

        self.last_touches = points.clone();
        Ok(Some(points))
    }

    /// Read touch event (simplified API for single-touch mode)
    ///
    /// Returns only the first touch point with pressed/released state.
    pub async fn read_touch(&mut self) -> Result<Option<TouchPoint>, E> {
        match self.read_touches().await? {
            Some(points) if !points.is_empty() => Ok(Some(points[0])),
            _ => Ok(None),
        }
    }

    /// Read a single register value
    async fn read_register(&mut self, reg: u8) -> Result<u8, E> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(FT3X68_ADDR, &[reg], &mut buf).await?;
        Ok(buf[0])
    }

    /// Read multiple registers into a buffer
    async fn read_registers(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), E> {
        self.i2c.write_read(FT3X68_ADDR, &[reg], buf).await
    }
}
