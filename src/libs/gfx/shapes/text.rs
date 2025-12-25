use core::cmp;

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{font_for_size, Charset, FontSize, MonoFont};
use crate::libs::gfx::Rasterizer;

/// Text shape with configurable font and per-instance styling.
pub struct Text<'a> {
    x: i32,
    y: i32,
    text: &'a str,
    color: Rgba8888,
    alpha: u8,
    font: MonoFont,
    letter_spacing_override: Option<i8>,
}

impl<'a> Text<'a> {
    pub fn new(x: i32, y: i32, text: &'a str) -> Self {
        Self {
            x,
            y,
            text,
            color: Rgba8888::rgb(255, 255, 255),
            alpha: 255,
            font: font_for_size(FontSize::Small),
            letter_spacing_override: None,
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

    pub fn font(mut self, font: MonoFont) -> Self {
        self.font = font;
        if let Some(letter_spacing) = self.letter_spacing_override {
            self.font = self.font.with_letter_spacing(letter_spacing);
        }
        self
    }

    pub fn size(mut self, size: FontSize) -> Self {
        self.font = font_for_size(size);
        if let Some(letter_spacing) = self.letter_spacing_override {
            self.font = self.font.with_letter_spacing(letter_spacing);
        }
        self
    }

    pub fn charsets(mut self, charsets: &'static [Charset]) -> Self {
        self.font = self.font.with_charsets(charsets);
        if let Some(letter_spacing) = self.letter_spacing_override {
            self.font = self.font.with_letter_spacing(letter_spacing);
        }
        self
    }

    pub fn letter_spacing(mut self, letter_spacing: i8) -> Self {
        self.letter_spacing_override = Some(letter_spacing);
        self.font = self.font.with_letter_spacing(letter_spacing);
        self
    }

    #[deprecated(note = "Use letter_spacing() instead")]
    pub fn tracking(self, tracking: i8) -> Self {
        self.letter_spacing(tracking)
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

        let advance = i32::from(self.font.advance());
        let spacing = i32::from(self.font.letter_spacing());
        let line_step = match self.letter_spacing_override {
            Some(adj) => i32::from(self.font.height()) + i32::from(cmp::max(adj, 0)),
            None => self.font.line_advance(),
        };

        let mut pen_x = self.x;
        let mut pen_y = self.y;
        let mut any_drawn = false;
        let mut min_drawn_x = i32::MAX;
        let mut max_drawn_x = i32::MIN;
        let mut min_drawn_y = i32::MAX;
        let mut max_drawn_y = i32::MIN;

        for ch in self.text.chars() {
            match ch {
                '\r' => continue,
                '\n' => {
                    pen_x = self.x;
                    pen_y += line_step;
                    continue;
                }
                _ => {}
            }

            let glyph = self.font.glyph(ch);
            let glyph_width = glyph.width() as i32;
            let glyph_height = glyph.height() as i32;
            let mut glyph_has_ink = false;

            for gy in 0..glyph_height {
                let py = pen_y + gy;
                if py < 0 || py >= height {
                    continue;
                }

                for gx in 0..glyph_width {
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
                min_drawn_y = min_drawn_y.min(pen_y);
                max_drawn_y = max_drawn_y.max(pen_y + glyph_height - 1);
            }

            pen_x += advance + spacing;
        }

        if any_drawn {
            let min_x = min_drawn_x.max(0);
            let max_x = max_drawn_x.min(width - 1);
            let min_y = min_drawn_y.max(0);
            let max_y = max_drawn_y.min(height - 1);
            rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
        }
    }
}
