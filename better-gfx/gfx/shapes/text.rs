// file: src/shapes/text.rs
// Fixed version - keeps original structure as much as possible, fixes lifetime issue

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{FONT_6X8, FONT_WIDTH, FONT_HEIGHT};
use crate::libs::gfx::{blend_rgb565, rgba8888_to_rgb565_and_alpha, Rasterizer};

/// Text shape that supports temporary strings by being lifetime-generic
pub struct Text<'a> {
    x: i32,
    y: i32,
    text: &'a str,
    color: Rgba8888,
    alpha: u8, // kept original additional alpha multiplier
}

impl<'a> Text<'a> {
    pub fn new(x: i32, y: i32, text: &'a str) -> Self {
        Self {
            x,
            y,
            text,
            color: Rgba8888::rgb(255, 255, 255),
            alpha: 255,
        }
    }

    pub fn color(mut self, color: Rgba8888) -> Self {
        self.color = color;
        self
    }

    pub fn alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }
}

impl<'a> super::Shape for Text<'a> {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        let (fg_rgb565, mut fg_alpha) = rgba8888_to_rgb565_and_alpha(self.color.to_u32());
        fg_alpha = ((fg_alpha as u32 * self.alpha as u32) / 255) as u8;

        if fg_alpha == 0 {
            return;
        }

        let width = rasterizer.width() as usize;
        let height = rasterizer.height() as i32;
        let mut buf = rasterizer.buffer_mut();

        let mut cursor_x = self.x;

        for c in self.text.bytes() {
            let idx = match c {
                b'0'..=b'9' => (c - b'0') as usize,
                b':' => 10,
                _ => continue,
            };

            let glyph = FONT_6X8[idx];

            for gy in 0..FONT_HEIGHT {
                let row = glyph[gy as usize];
                let py = self.y + gy;

                if py < 0 || py >= height {
                    continue;
                }

                for gx in 0..FONT_WIDTH {
                    if (row << gx) & 0x80 != 0 {
                        let px = cursor_x + gx;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }

                        let idx = (py as usize * width + px as usize) * 2;
                        let bg = ((buf[idx] as u16) << 8) | buf[idx + 1] as u16;
                        let out = blend_rgb565(bg, fg_rgb565, fg_alpha);
                        buf[idx] = (out >> 8) as u8;
                        buf[idx + 1] = out as u8;
                    }
                }
            }

            cursor_x += FONT_WIDTH + 1;
        }

        let min_x = self.x.max(0);
        let max_x = (cursor_x + FONT_WIDTH).min(rasterizer.width() as i32 - 1);
        let min_y = self.y.max(0);
        let max_y = (self.y + FONT_HEIGHT).min(rasterizer.height() as i32 - 1);
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}