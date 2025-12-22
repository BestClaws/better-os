// file: src/shapes/text.rs
// Fixed version - keeps original structure as much as possible, fixes lifetime issue

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{FONT_6X8, FONT_HEIGHT, FONT_WIDTH};
use crate::libs::gfx::Rasterizer;

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
        // Effective alpha combines color alpha with Text alpha
        let c = self.color.to_u32();
        let r = ((c >> 24) & 0xFF) as u8;
        let g = ((c >> 16) & 0xFF) as u8;
        let b = ((c >> 8) & 0xFF) as u8;
        let a = (c & 0xFF) as u8;
        let eff_a = ((a as u32 * self.alpha as u32) / 255) as u8;
        if eff_a == 0 {
            return;
        }
        let fg_rgba = Rgba8888::rgba(r, g, b, eff_a);

        let width = rasterizer.width() as i32;
        let height = rasterizer.height() as i32;

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

                        rasterizer.blend_pixel(px, py, fg_rgba, 255);
                    }
                }
            }

            cursor_x += FONT_WIDTH + 1;
        }

        let min_x = self.x.max(0);
        let max_x = (cursor_x + FONT_WIDTH).min(width - 1);
        let min_y = self.y.max(0);
        let max_y = (self.y + FONT_HEIGHT).min(height - 1);
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}
