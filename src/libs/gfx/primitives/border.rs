use crate::libs::gfx::core::color::{Color, ColorAlpha, Opacity};
use crate::libs::gfx::core::blend::{BlendDescriptor, BlendSource, BlendMode};
use crate::libs::gfx::core::geometry::Rect;
use crate::libs::gfx::draw_target::DrawTarget;
use crate::libs::gfx::layer::Layer;

/// Border side flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderSide {
    None = 0x00,
    Bottom = 0x01,
    Top = 0x02,
    Left = 0x04,
    Right = 0x08,
    Full = 0x0F,
}

impl BorderSide {
    pub fn has(self, side: BorderSide) -> bool {
        (self as u8) & (side as u8) != 0
    }
}

/// Border primitive builder
pub struct Border<'a> {
    layer: &'a mut Layer<'a>,
    rect: Rect,
    color: Color,
    width: i32,
    opacity: Opacity,
    radius: i32,
    side: BorderSide,
}

impl<'a> Border<'a> {
    pub fn new(layer: &'a mut Layer<'a>, rect: Rect) -> Self {
        Self {
            layer,
            rect,
            color: Color::WHITE,
            width: 1,
            opacity: Opacity::OPAQUE,
            radius: 0,
            side: BorderSide::Full,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(0);
        self
    }

    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = radius.max(0);
        self
    }

    pub fn side(mut self, side: BorderSide) -> Self {
        self.side = side;
        self
    }

    pub fn top(mut self) -> Self {
        self.side = BorderSide::Top;
        self
    }

    pub fn bottom(mut self) -> Self {
        self.side = BorderSide::Bottom;
        self
    }

    pub fn left(mut self) -> Self {
        self.side = BorderSide::Left;
        self
    }

    pub fn right(mut self) -> Self {
        self.side = BorderSide::Right;
        self
    }

    pub fn full(mut self) -> Self {
        self.side = BorderSide::Full;
        self
    }

    pub fn draw(self) {
        if self.width == 0 {
            return;
        }

        // Clamp radius to half of shortest side
        let short_side = self.rect.width.min(self.rect.height) as i32;
        let radius = self.radius.min(short_side / 2);

        if radius == 0 {
            self.draw_simple();
        } else {
            // For rounded borders, we'd need masking support
            // For now, fall back to simple borders
            self.draw_simple();
        }
    }

    fn draw_simple(self) {
        let outer = self.rect;
        
        // Calculate inner rectangle
        let left_offset = if self.side.has(BorderSide::Left) { self.width } else { 0 };
        let right_offset = if self.side.has(BorderSide::Right) { self.width } else { 0 };
        let top_offset = if self.side.has(BorderSide::Top) { self.width } else { 0 };
        let bottom_offset = if self.side.has(BorderSide::Bottom) { self.width } else { 0 };

        // Draw top border
        if self.side.has(BorderSide::Top) {
            let top_rect = Rect::new(outer.x, outer.y, outer.width, self.width as u32);
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: top_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }

        // Draw bottom border
        if self.side.has(BorderSide::Bottom) {
            let bottom_rect = Rect::new(
                outer.x,
                outer.y + outer.height as i32 - self.width,
                outer.width,
                self.width as u32
            );
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: bottom_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }

        // Draw left border (excluding corners already drawn)
        if self.side.has(BorderSide::Left) {
            let left_rect = Rect::new(
                outer.x,
                outer.y + top_offset,
                self.width as u32,
                (outer.height as i32 - top_offset - bottom_offset) as u32
            );
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: left_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }

        // Draw right border (excluding corners already drawn)
        if self.side.has(BorderSide::Right) {
            let right_rect = Rect::new(
                outer.x + outer.width as i32 - self.width,
                outer.y + top_offset,
                self.width as u32,
                (outer.height as i32 - top_offset - bottom_offset) as u32
            );
            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
                dest_rect: right_rect,
                opacity: self.opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
    }
}

impl<'a> Layer<'a> {
    pub fn border(&'a mut self, rect: Rect) -> Border<'a> {
        Border::new(self, rect)
    }
}
