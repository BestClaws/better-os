// Imports
use alloc::boxed::Box;
use defmt::export::u8;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Instant};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use embedded_graphics::{image::Image, pixelcolor::BinaryColor, prelude::*};
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
const FIRE_WIDTH: i32 = 4;
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
    ship_x: i32,
    bullets: heapless::Vec<(i32, f32), 10>,
    enemy_bullets: heapless::Vec<(i32, f32), 10>,
    enemy: Option<(i32, i32)>,
    enemy_dx: i32,
    last_fire_toggle: Instant,
    fire_left: bool,
    last_enemy_spawn: Instant,
    spawn_counter: usize,
    last_update: Instant,
    dead: bool,
    random_index: usize,
    last_enemy_fire: Instant,
    rng: PiRng,
}

impl Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {
        let i2c_disp = I2CDisplayInterface::new(i2c_1);
        let display = Ssd1306Async::new(i2c_disp, DisplaySize128x64, DisplayRotation::Rotate180)
            .into_buffered_graphics_mode();

        Self {
            display,
            ship_x: DISPLAY_WIDTH / 2,
            bullets: heapless::Vec::new(),
            enemy_bullets: heapless::Vec::new(),
            enemy: None,
            enemy_dx: 2,
            last_fire_toggle: Instant::now(),
            fire_left: true,
            last_enemy_spawn: Instant::now(),
            spawn_counter: 0,
            last_update: Instant::now(),
            dead: false,
            random_index: 0,
            last_enemy_fire: Instant::now(),
            rng: PiRng::new()

        }
    }

    fn reset(&mut self) {
        self.ship_x = DISPLAY_WIDTH / 2;
        self.bullets.clear();
        self.enemy_bullets.clear();
        self.enemy = None;
        self.dead = false;
        self.spawn_counter = 0;
        self.random_index = 0;
        self.last_update = Instant::now();
        self.last_enemy_spawn = Instant::now();
        self.last_fire_toggle = Instant::now();
        self.last_enemy_fire = Instant::now();
    }



    fn update_game_state(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_update;
        let delta_secs = delta.as_millis() as f32 / 1000.0;
        self.last_update = now;

        for i in (0..self.bullets.len()).rev() {
            let (x, y) = self.bullets[i];
            let new_y = y - BULLET_SPEED * delta_secs;
            if new_y < 0.0 {
                self.bullets.swap_remove(i);
            } else {
                self.bullets[i] = (x, new_y);
            }
        }

        for i in (0..self.enemy_bullets.len()).rev() {
            let (x, y) = self.enemy_bullets[i];
            let new_y = y + BULLET_SPEED * delta_secs;
            if new_y > DISPLAY_HEIGHT as f32 {
                self.enemy_bullets.swap_remove(i);
            } else {
                self.enemy_bullets[i] = (x, new_y);
            }
        }

        if (now - self.last_fire_toggle).as_millis() >= FIRE_TOGGLE_MS {
            self.fire_left = !self.fire_left;
            self.last_fire_toggle = now;
        }

        if self.enemy.is_none() && (now - self.last_enemy_spawn) >= Duration::from_millis(ENEMY_SPAWN_INTERVAL) {
            let positions = [ENEMY_WIDTH / 2 + 20, DISPLAY_WIDTH / 2, DISPLAY_WIDTH - ENEMY_WIDTH / 2 - 20];
            let x = positions[self.spawn_counter % positions.len()];
            self.enemy = Some((x, ENEMY_HEIGHT / 2));
            self.enemy_dx = if self.spawn_counter % 2 == 0 { 2 } else { -2 };
            self.last_enemy_spawn = now;
            self.spawn_counter = self.spawn_counter.wrapping_add(1);
        }

        if let Some((x, y)) = self.enemy {
            let step_size = (self.rng.next_in_range(100) % 5) as i32 + 1; // Random step between 1 and 5
            let direction = if self.rng.next_in_range(100) % 2 == 0 { -1 } else { 1 }; // Randomly choose left or right
            self.enemy_dx = step_size * direction;
            let new_x = (x + self.enemy_dx).clamp(ENEMY_WIDTH / 2, DISPLAY_WIDTH - ENEMY_WIDTH / 2);
            self.enemy = Some((new_x, y));

            if (now - self.last_enemy_fire).as_millis() >= ENEMY_FIRE_INTERVAL_MS {
                self.last_enemy_fire = now;
                if self.enemy_bullets.len() < self.enemy_bullets.capacity() {
                    self.enemy_bullets.push((new_x, y as f32 + ENEMY_HEIGHT as f32 / 2.0)).unwrap();
                }
            }
        }

        if let Some((enemy_x, enemy_y)) = self.enemy {
            for i in (0..self.bullets.len()).rev() {
                let (bullet_x, bullet_y) = self.bullets[i];
                if (bullet_x - enemy_x).abs() < (BULLET_WIDTH + ENEMY_WIDTH) / 2 &&
                    (bullet_y as i32 - enemy_y).abs() < (BULLET_HEIGHT + ENEMY_HEIGHT) / 2 {
                    self.bullets.swap_remove(i);
                    self.enemy = None;
                }
            }
        }

        for &(bx, by) in &self.enemy_bullets {
            if (bx - self.ship_x).abs() < (BULLET_WIDTH + SHIP_WIDTH) / 2 &&
                (by as i32 - (DISPLAY_HEIGHT - SHIP_HEIGHT / 2)).abs() < (BULLET_HEIGHT + SHIP_HEIGHT) / 2 {
                self.dead = true;
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl AsyncDisplay for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
    }

    async fn draw(&mut self, event: HumanInputEvent) {
        if self.dead {
            if let HumanInputEvent::OK_PRESSED = event {
                self.reset();
            }
        } else {
            match event {
                HumanInputEvent::NavUp => {
                    self.ship_x = (self.ship_x - SHIP_SPEED).max(SHIP_WIDTH / 2);
                },
                HumanInputEvent::NavDown => {
                    self.ship_x = (self.ship_x + SHIP_SPEED).min(DISPLAY_WIDTH - SHIP_WIDTH / 2);
                },
                HumanInputEvent::OK_PRESSED => {
                    if self.bullets.len() < self.bullets.capacity() {
                        self.bullets.push((self.ship_x, (DISPLAY_HEIGHT - SHIP_HEIGHT - FIRE_HEIGHT) as f32)).unwrap();
                    }
                },
                _ => {}
            }
        }

        self.update_game_state();
        self.display.clear(BinaryColor::Off).unwrap();

        if self.dead {
            for i in 0..5 {
                let offset = (i as i32 - 2) * 5;
                let bmp = if self.fire_left { Bmp::from_slice(FIRE_LEFT).unwrap() } else { Bmp::from_slice(FIRE_RIGHT).unwrap() };
                let img = Image::with_center(&bmp, Point::new(self.ship_x + offset, DISPLAY_HEIGHT - SHIP_HEIGHT - FIRE_HEIGHT / 2));
                img.draw(&mut self.display).unwrap();
            }
        } else {
            let ship_bmp = Bmp::from_slice(SHIP).unwrap();
            let ship_img = Image::with_center(&ship_bmp, Point::new(self.ship_x, DISPLAY_HEIGHT - SHIP_HEIGHT / 2));
            ship_img.draw(&mut self.display).unwrap();

            let fire_bmp = if self.fire_left { Bmp::from_slice(FIRE_LEFT).unwrap() } else { Bmp::from_slice(FIRE_RIGHT).unwrap() };
            let fire_img = Image::with_center(&fire_bmp, Point::new(self.ship_x, DISPLAY_HEIGHT - SHIP_HEIGHT - FIRE_HEIGHT / 2));
            fire_img.draw(&mut self.display).unwrap();
        }

        let bullet_bmp = Bmp::from_slice(BULLET).unwrap();
        for &(x, y) in &self.bullets {
            let img = Image::with_center(&bullet_bmp, Point::new(x, y as i32));
            img.draw(&mut self.display).unwrap();
        }

        for &(x, y) in &self.enemy_bullets {
            let img = Image::with_center(&bullet_bmp, Point::new(x, y as i32));
            img.draw(&mut self.display).unwrap();
        }

        if let Some((x, y)) = self.enemy {
            let enemy_bmp = Bmp::from_slice(ENEMY).unwrap();
            let enemy_img = Image::with_center(&enemy_bmp, Point::new(x, y));
            enemy_img.draw(&mut self.display).unwrap();
        }

        self.display.flush().await.unwrap();
    }
}