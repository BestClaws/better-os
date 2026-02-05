//! CST816S Capacitive Touch Controller Driver
//!
//! This driver supports the CST816S touch controller from Hynitron,
//! commonly used in smartwatch displays. Features include:
//! - Single-touch detection
//! - Gesture recognition
//! - I2C interface
//! - Interrupt-driven or polling mode
//!
//! # Hardware Specifications
//! - Interface: I2C (address 0x15)
//! - Max Touch Points: 1
//! - Resolution: Up to 240x240 (configurable)
//! - Gestures: Swipe (up/down/left/right), long press, double tap
//!
//! # Usage
//! ```rust,no_run
//! let touch = Cst816s::new(i2c, int_pin, rst_pin);
//! touch.init().await?;
//!
//! loop {
//!     if let Some(event) = touch.read_touch().await {
//!         match event {
//!             TouchEvent::Press { x, y } => { /* handle */ },
//!             TouchEvent::Release => { /* handle */ },
//!         }
//!     }
//! }
//! ```

extern crate alloc;

use defmt::{debug, error, info, Format};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

// ═══════════════════════════════════════════════════════════════════════════
// Hardware Configuration & Constants
// ═══════════════════════════════════════════════════════════════════════════

/// I2C device address for CST816S (7-bit address)
const CST816S_ADDR: u8 = 0x15;

/// Register addresses
const REG_GESTURE_ID: u8 = 0x01;
const REG_FINGER_NUM: u8 = 0x02;
const REG_XPOS_H: u8 = 0x03;
const REG_XPOS_L: u8 = 0x04;
const REG_YPOS_H: u8 = 0x05;
const REG_YPOS_L: u8 = 0x06;
const REG_CHIP_ID: u8 = 0xA7;
const REG_FW_VERSION: u8 = 0xA9;

/// Expected chip ID for CST816S
const CHIP_ID_CST816S: u8 = 0xB4;

/// Gesture IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum Gesture {
    None = 0x00,
    SwipeDown = 0x01,
    SwipeUp = 0x02,
    SwipeLeft = 0x03,
    SwipeRight = 0x04,
    SingleClick = 0x05,
    DoubleClick = 0x0B,
    LongPress = 0x0C,
}

impl From<u8> for Gesture {
    fn from(val: u8) -> Self {
        match val {
            0x01 => Gesture::SwipeDown,
            0x02 => Gesture::SwipeUp,
            0x03 => Gesture::SwipeLeft,
            0x04 => Gesture::SwipeRight,
            0x05 => Gesture::SingleClick,
            0x0B => Gesture::DoubleClick,
            0x0C => Gesture::LongPress,
            _ => Gesture::None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Touch Event Types
// ═══════════════════════════════════════════════════════════════════════════

/// Touch event from the controller
#[derive(Debug, Clone, Copy, Format)]
pub struct TouchPoint {
    /// X coordinate (0 to display width)
    pub x: u16,
    /// Y coordinate (0 to display height)
    pub y: u16,
    /// Whether finger is touching (true) or released (false)
    pub pressed: bool,
    /// Detected gesture, if any
    pub gesture: Gesture,
}

// ═══════════════════════════════════════════════════════════════════════════
// Driver Implementation
// ═══════════════════════════════════════════════════════════════════════════

/// CST816S touch controller driver
pub struct Cst816s<I2C> {
    i2c: I2C,
    last_touch: Option<TouchPoint>,
}

impl<I2C, E> Cst816s<I2C>
where
    I2C: I2c<Error = E>,
    E: defmt::Format,
{
    /// Create a new CST816S driver
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            last_touch: None,
        }
    }

    /// Initialize the touch controller
    ///
    /// Verifies the chip ID and reads firmware version.
    pub async fn init(&mut self) -> Result<(), E> {
        info!("Initializing CST816S touch controller");

        // Wait for controller to be ready
        Timer::after_millis(50).await;

        // Read chip ID
        let chip_id = self.read_register(REG_CHIP_ID).await?;
        if chip_id != CHIP_ID_CST816S {
            error!("CST816S chip ID mismatch: expected 0xB4, got 0x{:02x}", chip_id);
            // Don't fail - some variants have different IDs
        }

        // Read firmware version
        let fw_version = self.read_register(REG_FW_VERSION).await?;
        info!("CST816S initialized: chip_id=0x{:02x} fw_version=0x{:02x}", chip_id, fw_version);

        Ok(())
    }

    /// Read a single touch event from the controller
    ///
    /// Returns None if no touch is detected, or Some(TouchPoint) with coordinates.
    pub async fn read_touch(&mut self) -> Result<Option<TouchPoint>, E> {
        // Read gesture and finger count
        let gesture_id = self.read_register(REG_GESTURE_ID).await?;
        let finger_num = self.read_register(REG_FINGER_NUM).await?;

        // No fingers touching
        if finger_num == 0 {
            if self.last_touch.is_some() {
                // Generate release event
                let mut released = self.last_touch.unwrap();
                released.pressed = false;
                self.last_touch = None;
                return Ok(Some(released));
            }
            return Ok(None);
        }

        // Read touch coordinates (12-bit values split across 2 registers)
        let x_h = self.read_register(REG_XPOS_H).await?;
        let x_l = self.read_register(REG_XPOS_L).await?;
        let y_h = self.read_register(REG_YPOS_H).await?;
        let y_l = self.read_register(REG_YPOS_L).await?;

        let x = (((x_h & 0x0F) as u16) << 8) | (x_l as u16);
        let y = (((y_h & 0x0F) as u16) << 8) | (y_l as u16);

        let touch = TouchPoint {
            x,
            y,
            pressed: true,
            gesture: Gesture::from(gesture_id),
        };

        self.last_touch = Some(touch);
        Ok(Some(touch))
    }

    /// Read a single register value
    async fn read_register(&mut self, reg: u8) -> Result<u8, E> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(CST816S_ADDR, &[reg], &mut buf).await?;
        Ok(buf[0])
    }

    /// Read multiple registers into a buffer
    async fn read_registers(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), E> {
        self.i2c.write_read(CST816S_ADDR, &[reg], buf).await
    }
}
