use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{MonoFont, SYSTEM_MONO_FONT};
use crate::libs::gfx::Rasterizer;

/// Text shape with configurable font and per-instance styling.
pub struct Text<'a> {
    x: i32,
    y: i32,
    text: &'a str,
    color: Rgba8888,
    alpha: u8,
    font: &'static MonoFont,
}

impl<'a> Text<'a> {
    pub fn new(x: i32, y: i32, text: &'a str) -> Self {
        Self {
            x,
            y,
            text,
            color: Rgba8888::rgb(255, 255, 255),
            alpha: 255,
            font: &SYSTEM_MONO_FONT,
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

    pub fn font(mut self, font: &'static MonoFont) -> Self {
        self.font = font;
        self
    }
}

impl<'a> super::Shape for Text<'a> {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        let packed = self.color.to_u32();
        let r = ((packed >> 24) & 0xFF) as u8;
        let g = ((packed >> 16) & 0xFF) as u8;
        let b = ((packed >> 8) & 0xFF) as u8;
        let a = (packed & 0xFF) as u8;

        let effective_alpha = ((a as u32 * self.alpha as u32) / 255) as u8;
        if effective_alpha == 0 {
            return;
        }

        let fg = Rgba8888::rgba(r, g, b, effective_alpha);
        let width = rasterizer.width() as i32;
        let height = rasterizer.height() as i32;

        let glyph_height = self.font.height() as i32;
        let glyph_width = self.font.width() as i32;
        let advance = self.font.advance() as i32;
        let tracking = self.font.tracking() as i32;

        let mut pen_x = self.x;
        let mut any_drawn = false;
        let mut min_drawn_x = i32::MAX;
        let mut max_drawn_x = i32::MIN;

        for ch in self.text.chars() {
            if let Some(glyph) = self.font.glyph(ch) {
                let mut glyph_has_ink = false;

                for gy in 0..glyph.height() as i32 {
                    let py = self.y + gy;
                    if py < 0 || py >= height {
                        continue;
                    }

                    for gx in 0..glyph.width() as i32 {
                        let px = pen_x + gx;
                        if px < 0 || px >= width {
                            continue;
                        }

                        if glyph.is_pixel_inked(gx as u8, gy as u8) {
                            rasterizer.blend_pixel(px, py, fg, 255);
                            glyph_has_ink = true;
                        }
                    }
                }

                if glyph_has_ink {
                    any_drawn = true;
                    min_drawn_x = min_drawn_x.min(pen_x);
                    max_drawn_x = max_drawn_x.max(pen_x + glyph_width - 1);
                }
            }

            pen_x += advance + tracking;
        }

        if any_drawn {
            let min_x = min_drawn_x.max(0);
            let max_x = max_drawn_x.min(width - 1);
            let min_y = self.y.max(0);
            let max_y = (self.y + glyph_height - 1).min(height - 1);
            rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
        }
    }
}
