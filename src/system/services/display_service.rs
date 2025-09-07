use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::geometry::{Point as EgPoint, Size as EgSize};
use defmt::info;

use crate::system::hal::display::AsyncDisplay;
use crate::libs::gfx::two_d::Rect;
use crate::system::kernel::config::resources::{FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, FRAME_SCALE_FACTOR};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb565,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self { PixelFormat::Rgb565 => 2 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DisplayCapabilities {
    pub supported_formats: &'static [PixelFormat],
    pub preferred_format: PixelFormat,
}

pub struct DisplayService {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    mode: PixelFormat,
    scale: u32,
    width: u32,
    height: u32,
}

impl DisplayService {
    pub fn new(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
        boot_mode: Option<PixelFormat>,
        scale: Option<u32>,
    ) -> Self {
        // For now use compile-time geometry/scale
        let width = FRAME_BUFFER_WIDTH;
        let height = FRAME_BUFFER_HEIGHT;
        let scale = scale.unwrap_or(FRAME_SCALE_FACTOR);

        let caps = DisplayCapabilities {
            supported_formats: &[PixelFormat::Rgb565],
            preferred_format: PixelFormat::Rgb565,
        };

        let chosen = boot_mode.unwrap_or(caps.preferred_format);
        // Avoid requiring defmt::Format on PixelFormat by logging as integer
        let mode_u8 = match chosen { PixelFormat::Rgb565 => 1 };
        info!("DisplayService init: mode={} scale={} size={}x{}", mode_u8, scale, width, height);

        Self { driver, mode: chosen, scale, width, height }
    }

    pub fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities {
            supported_formats: &[PixelFormat::Rgb565],
            preferred_format: PixelFormat::Rgb565,
        }
    }

    pub fn pixel_format(&self) -> PixelFormat { self.mode }
    pub fn scale(&self) -> u32 { self.scale }
    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }

    pub fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * self.mode.bytes_per_pixel()
    }

    pub async fn draw_full(&self, buffer: &[u8]) {
        let mut l = self.driver.lock().await;
        l.draw(buffer, self.scale).await;
    }

    pub async fn draw_region(&self, buffer: &[u8], region: Rect) {
        let mut l = self.driver.lock().await;
        l.draw_region(
            buffer,
            Rectangle::new(
                EgPoint::new(region.top_left.x, region.top_left.y),
                EgSize::new(region.size.width, region.size.height),
            ),
            self.scale,
        ).await;
    }
}


