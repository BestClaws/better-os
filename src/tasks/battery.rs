use embassy_sync::semaphore::Semaphore;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Point, Primitive, Size};
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics_framebuf::FrameBuf;
use crate::{
    system::resources::framebuffer::{request_framebuffer, FB_SEMAPHORE, FRAME_CHANNEL, SubmitFrame},
};

#[embassy_executor::task]
pub async fn battery() {
    // Wait for a free framebuffer
    FB_SEMAPHORE.acquire(1).await.unwrap();

    if let Some(fb) = request_framebuffer() {
        draw_ui(fb.buf);

        FRAME_CHANNEL.sender().send(SubmitFrame {
            id: fb.id,
            app_id: 2, // unique per app
        }).await;


        core::mem::forget(fb); // compositor owns & drops it
    }
}


pub fn draw_ui(buf: &mut [u8; 1024]) {
    // Clear buffer first
    buf.fill(0);

    let mut fb = BitPackedFramebuffer {
        buf,
        width: 128,
        height: 64,
    };

    // Border
    Rectangle::new(Point::zero(), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut fb)
        .unwrap();

    // Text
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::new("better-os UI", Point::new(28, 28), style)
        .draw(&mut fb)
        .unwrap();
}

use embedded_graphics::{
    prelude::*,
    draw_target::DrawTarget,
    geometry::{OriginDimensions},
};

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
