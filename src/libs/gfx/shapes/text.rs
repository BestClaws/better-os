use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{font_for_size, Charset, FontSize, MonoFont};
use crate::libs::gfx::compat::color::IntoEgRgb;
use crate::libs::gfx::compat::draw_target::RasterizerDrawTarget;
use crate::libs::gfx::Rasterizer;
use embedded_graphics::geometry::Point as EgPoint;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::text::{Baseline as EgBaseline, Text as EgText};
use embedded_graphics::Drawable;

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
        let a = (packed & 0xFF) as u8;

        let effective_alpha = ((a as u32 * self.alpha as u32) / 255) as u8;
        if effective_alpha == 0 {
            return;
        }
        let mut target = RasterizerDrawTarget::new(rasterizer, effective_alpha);

        let text_color = self.color.into_rgb888();
        let style = MonoTextStyle::new(self.font.embedded(), text_color);

        let top_left = EgPoint::new(self.x, self.y);
        let _ = EgText::with_baseline(self.text, top_left, style, EgBaseline::Top).draw(&mut target);

        target.finish();
    }
}
