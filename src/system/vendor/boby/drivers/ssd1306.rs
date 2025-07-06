use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, };
use embassy_time::{Duration, Instant};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use crate::system::hal::display::AsyncDisplay;
use embedded_graphics::{
    image::Image,
    pixelcolor::BinaryColor,
    prelude::*,
};
use tinybmp::Bmp;
use crate::system::services::human_input::HumanInputEvent;

const FIRE_LEFT: &[u8] = include_bytes!("../../../../assets/fire_left.bmp");
const FIRE_RIGHT: &[u8] = include_bytes!("../../../../assets/fire_right.bmp");
const SHIP: &[u8] = include_bytes!("../../../../assets/ship.bmp");
const ENEMY: &[u8] = include_bytes!("../../../../assets/enemy.bmp");
const BULLET: &[u8] = include_bytes!("../../../../assets/bullet.bmp");

const DISPLAY_WIDTH: i32 = 128;
const DISPLAY_HEIGHT: i32 = 64;
const SHIP_WIDTH: i32 = 34;
const SHIP_HEIGHT: i32 = 15;
const ENEMY_WIDTH: i32 = 23;
const ENEMY_HEIGHT: i32 = 12;
const BULLET_WIDTH: i32 = 4;
const BULLET_HEIGHT: i32 = 5;
const FIRE_WIDTH: i32 = 4;
const FIRE_HEIGHT: i32 = 4;
const SHIP_SPEED: i32 = 5; // pixels per scroll
const BULLET_SPEED: f32 = 8.0; // pixels per second
const FIRE_TOGGLE_MS: u64 = 15; // toggle fire animation every 15ms
const ENEMY_SPAWN_INTERVAL: u64 = 3000; // spawn every 3 seconds

pub(crate) struct Ssd1306Driver {
    display: Ssd1306Async<I2CInterface<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
    ship_x: i32, // ship's x position (center)
    bullets: heapless::Vec<(i32, f32), 10>, // (x, y) positions of bullets
    enemy: Option<(i32, i32)>, // (x, y) position of enemy
    last_fire_toggle: Instant, // last time fire animation toggled
    fire_left: bool, // true for fire_left, false for fire_right
    last_enemy_spawn: Instant, // last time an enemy was spawned
    spawn_counter: usize, // tracks spawn pattern
    last_update: Instant, // last time the game state was updated
}

// impl Driver for Ssd1306Driver {}

impl  Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {


        let i2c_disp = I2CDisplayInterface::new(i2c_1);


        let display = Ssd1306Async::new(
            i2c_disp,
            DisplaySize128x64,
            DisplayRotation::Rotate180,
        ).into_buffered_graphics_mode();


        Self {
            display,
            ship_x: DISPLAY_WIDTH / 2,
            bullets: heapless::Vec::new(),
            enemy: None,
            last_fire_toggle: Instant::now(),
            fire_left: true,
            last_enemy_spawn: Instant::now(),
            spawn_counter: 0,
            last_update: Instant::now(),
        }

        // display.init().await.unwrap();

    }


    fn update_game_state(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_update;
        let delta_secs = delta.as_millis() as f32 / 1000.0;
        self.last_update = now;

        // Update bullets
        for i in (0..self.bullets.len()).rev() {
            let (x, y) = self.bullets[i];
            let new_y = y - BULLET_SPEED * delta_secs;
            if new_y < 0.0 {
                self.bullets.swap_remove(i);
            } else {
                self.bullets[i] = (x, new_y);
            }
        }

        // Toggle fire animation
        if (now - self.last_fire_toggle).as_millis() >= FIRE_TOGGLE_MS {
            self.fire_left = !self.fire_left;
            self.last_fire_toggle = now;
        }

        // Spawn new enemy if needed
        if self.enemy.is_none() && (now - self.last_enemy_spawn) >= Duration::from_millis(ENEMY_SPAWN_INTERVAL) {
            // Deterministic spawn pattern: left, center, right
            const SPAWN_POSITIONS: [i32; 3] = [
                ENEMY_WIDTH / 2 + 20,              // left
                DISPLAY_WIDTH / 2,                 // center
                DISPLAY_WIDTH - ENEMY_WIDTH / 2 - 20, // right
            ];
            let x = SPAWN_POSITIONS[self.spawn_counter % SPAWN_POSITIONS.len()];
            self.enemy = Some((x, ENEMY_HEIGHT / 2));
            self.last_enemy_spawn = now;
            self.spawn_counter = self.spawn_counter.wrapping_add(1);
        }

        // Check for bullet-enemy collisions
        if let Some((enemy_x, enemy_y)) = self.enemy {
            for i in (0..self.bullets.len()).rev() {
                let (bullet_x, bullet_y) = self.bullets[i];
                if (bullet_x - enemy_x).abs() < (BULLET_WIDTH + ENEMY_WIDTH) / 2 &&
                    (bullet_y as i32 - enemy_y).abs() < (BULLET_HEIGHT + ENEMY_HEIGHT) / 2 {
                    self.bullets.swap_remove(i);
                    self.enemy = None; // Enemy destroyed
                }
            }
        }
    }
}






#[async_trait(?Send)]
impl AsyncDisplay for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
    }

    async fn draw(&mut self, event: HumanInputEvent) {
        // Handle input
        match event {
            HumanInputEvent::NavUp => {
                info!("Moving left");
                self.ship_x = (self.ship_x - SHIP_SPEED).max(SHIP_WIDTH / 2);
            },
            HumanInputEvent::NavDown => {
                info!("Moving right");
                self.ship_x = (self.ship_x + SHIP_SPEED).min(DISPLAY_WIDTH - SHIP_WIDTH / 2);
            },
            HumanInputEvent::OK_PRESSED => {
                info!("Shooting bullet");
                if self.bullets.len() < self.bullets.capacity() {
                    self.bullets.push((self.ship_x, (DISPLAY_HEIGHT - SHIP_HEIGHT - FIRE_HEIGHT) as f32)).unwrap();
                }
            },
            HumanInputEvent::OK_RELEASED | HumanInputEvent::OK_HELD => {},
        }

        // Update game state
        self.update_game_state();

        // Clear display
        self.display.clear(BinaryColor::Off).unwrap();

        // Draw ship
        let ship_bmp = Bmp::from_slice(SHIP).unwrap();
        let ship_image = Image::with_center(
            &ship_bmp,
            Point::new(self.ship_x, DISPLAY_HEIGHT - SHIP_HEIGHT / 2)
        );
        ship_image.draw(&mut self.display).unwrap();

        // Draw fire
        let fire_bmp = if self.fire_left {
            Bmp::from_slice(FIRE_LEFT).unwrap()
        } else {
            Bmp::from_slice(FIRE_RIGHT).unwrap()
        };
        let fire_image = Image::with_center(
            &fire_bmp,
            Point::new(self.ship_x, DISPLAY_HEIGHT - SHIP_HEIGHT - FIRE_HEIGHT / 2)
        );
        fire_image.draw(&mut self.display).unwrap();

        // Draw bullets
        let bullet_bmp = Bmp::from_slice(BULLET).unwrap();
        for &(x, y) in &self.bullets {
            let bullet_image = Image::with_center(&bullet_bmp, Point::new(x, y as i32));
            bullet_image.draw(&mut self.display).unwrap();
        }

        // Draw enemy
        if let Some((x, y)) = self.enemy {
            let enemy_bmp = Bmp::from_slice(ENEMY).unwrap();
            let enemy_image = Image::with_center(&enemy_bmp, Point::new(x, y));
            enemy_image.draw(&mut self.display).unwrap();
        }

        // Flush display
        self.display.flush().await.unwrap();
    }

}
