// Imports
use alloc::boxed::Box;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Instant};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use embedded_graphics::{image::Image, pixelcolor::BinaryColor, prelude::*};
use esp_hal::sha::Digest;
use tinybmp::Bmp;
use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input::HumanInputEvent;

// Assets
const FIRE_LEFT: &[u8] = include_bytes!("../../../../assets/fire_left.bmp");
const FIRE_RIGHT: &[u8] = include_bytes!("../../../../assets/fire_right.bmp");
const SHIP: &[u8] = include_bytes!("../../../../assets/ship.bmp");
const ENEMY: &[u8] = include_bytes!("../../../../assets/enemy.bmp");
const BULLET: &[u8] = include_bytes!("../../../../assets/bullet.bmp");

// Display and game constants
const DISPLAY_WIDTH: i32 = 128;
const DISPLAY_HEIGHT: i32 = 64;
const SHIP_WIDTH: i32 = 23;
const SHIP_HEIGHT: i32 = 12;
const ENEMY_WIDTH: i32 = 23;
const ENEMY_HEIGHT: i32 = 12;
const BULLET_WIDTH: i32 = 4;
const BULLET_HEIGHT: i32 = 5;
const FIRE_HEIGHT: i32 = 4;
const SHIP_SPEED: i32 = 5;
const BULLET_SPEED: f32 = 16.0;
const FIRE_TOGGLE_MS: u64 = 15;
const ENEMY_SPAWN_INTERVAL: u64 = 3000;
const ENEMY_FIRE_INTERVAL_MS: u64 = 4000; // Added for slower enemy fire rate

// Predefined pseudo-random values
const PI_HEX_BYTES: [u8; 64] = [
    0x24, 0x3F, 0x6A, 0x88, 0x85, 0xA3, 0x08, 0xD3,
    0x13, 0x19, 0x8A, 0x2E, 0x03, 0x70, 0x73, 0x44,
    0xA4, 0x09, 0x38, 0x22, 0x29, 0x9F, 0x31, 0xD0,
    0x08, 0x2E, 0xFA, 0x98, 0xEC, 0x4E, 0x6C, 0x89,
    0x4D, 0xEF, 0x95, 0x19, 0x9D, 0x4F, 0xB8, 0x1B,
    0x10, 0xFA, 0x7B, 0xBD, 0x9A, 0xC2, 0x87, 0x62,
    0x34, 0x3F, 0x68, 0x2A, 0xEB, 0xC0, 0x1E, 0x16,
    0x6E, 0x6F, 0xE1, 0x4A, 0xF7, 0x23, 0xA9, 0x98,
];

pub struct PiRng {
    index: usize,
}

impl PiRng {
    pub const fn new() -> Self {
        Self { index: 0 }
    }

    pub fn next_u8(&mut self) -> u8 {
        let val = PI_HEX_BYTES[self.index % PI_HEX_BYTES.len()];
        self.index = self.index.wrapping_add(1);
        val
    }

    pub fn next_in_range(&mut self, max: u8) -> u8 {
        self.next_u8() % max
    }
}

pub(crate) struct Ssd1306Driver {
    display: Ssd1306Async<I2CInterface<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,

}

impl Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {
        let i2c_display = I2CDisplayInterface::new(i2c_1);
        let display = Ssd1306Async::new(i2c_display, DisplaySize128x64, DisplayRotation::Rotate180)
            .into_buffered_graphics_mode();

        Self {
            display,
        }
    }

}

#[async_trait::async_trait(?Send)]
impl AsyncDisplay for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
    }

    async fn draw(&mut self, buffer: &[u8]) {
        self.display.draw(buffer).await;
        self.display.flush().await;
    }

}