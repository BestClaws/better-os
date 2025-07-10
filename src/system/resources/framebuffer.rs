use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore};
use portable_atomic::{AtomicU32, Ordering};

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
pub const FRAME_SIZE: usize = WIDTH * HEIGHT / 8;
pub const NUM_BUFFERS: usize = 2;

pub static mut FRAMEBUFFERS: [[u8; FRAME_SIZE]; NUM_BUFFERS] = [[0; FRAME_SIZE]; NUM_BUFFERS];
pub static BUFFER_USED: AtomicU32 = AtomicU32::new(0);
pub static FB_SEMAPHORE: GreedySemaphore<CriticalSectionRawMutex> =
    GreedySemaphore::new(NUM_BUFFERS);

pub struct Framebuffer {
    pub id: usize,
    pub buf: &'static mut [u8; FRAME_SIZE],
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        BUFFER_USED.fetch_and(!(1 << self.id), Ordering::SeqCst);
        FB_SEMAPHORE.release(1);
    }
}

pub fn request_framebuffer() -> Option<Framebuffer> {
    for id in 0..NUM_BUFFERS {
        let mask = 1 << id;
        if BUFFER_USED.fetch_or(mask, Ordering::SeqCst) & mask == 0 {
            unsafe {
                return Some(Framebuffer {
                    id,
                    buf: &mut FRAMEBUFFERS[id],
                });
            }
        }
    }
    None
}


use embassy_sync::channel::Channel;


#[derive(Clone, Copy)]
pub struct SubmitFrame {
    pub id: usize,
    pub app_id: usize,
}


pub static FRAME_CHANNEL: Channel<CriticalSectionRawMutex, SubmitFrame, NUM_BUFFERS> =
    Channel::new();


use embedded_graphics::{
    prelude::*,
    draw_target::DrawTarget,
    geometry::{OriginDimensions},
};
use embedded_graphics::pixelcolor::BinaryColor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::services::battery::BATTERY_CHANNEL;

pub struct BitPackedFramebuffer<'a> {
    pub buf: &'a mut [u8; 1024],
    pub width: u32,
    pub height: u32,
}

impl<'a> OriginDimensions for BitPackedFramebuffer<'a> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl<'a> DrawTarget for BitPackedFramebuffer<'a> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                continue;
            }

            let x = x as usize;
            let y = y as usize;

            // SSD1306 expects vertical bit layout: each byte = 8 vertical pixels
            let byte_index = x + (y / 8) * self.width as usize;
            let bit_index = y % 8;

            match color {
                BinaryColor::On => self.buf[byte_index] |= 1 << bit_index,
                BinaryColor::Off => self.buf[byte_index] &= !(1 << bit_index),
            }
        }

        Ok(())
    }
}
