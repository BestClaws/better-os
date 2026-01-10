use crate::libs::gfx::core::color::{Color, ColorAlpha, Opacity};
use crate::libs::gfx::core::blend::{BlendDescriptor, BlendSource, BlendMode};
use crate::libs::gfx::core::geometry::Rect;
use crate::libs::gfx::draw_target::DrawTarget;
use crate::libs::gfx::layer::Layer;

/// Box shadow primitive builder
pub struct BoxShadow<'a> {
    layer: &'a mut Layer<'a>,
    rect: Rect,
    color: Color,
    width: i32,        // Blur radius
    spread: i32,       // Expand/contract shadow
    offset_x: i32,
    offset_y: i32,
    opacity: Opacity,
    radius: i32,
}

impl<'a> BoxShadow<'a> {
    pub fn new(layer: &'a mut Layer<'a>, rect: Rect) -> Self {
        Self {
            layer,
            rect,
            color: Color::BLACK,
            width: 5,
            spread: 0,
            offset_x: 0,
            offset_y: 0,
            opacity: Opacity::new(128),
            radius: 0,
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

    pub fn spread(mut self, spread: i32) -> Self {
        self.spread = spread;
        self
    }

    pub fn offset(mut self, x: i32, y: i32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    pub fn offset_x(mut self, x: i32) -> Self {
        self.offset_x = x;
        self
    }

    pub fn offset_y(mut self, y: i32) -> Self {
        self.offset_y = y;
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

    pub fn draw(mut self) {
        if self.width == 0 {
            return;
        }

        // Calculate shadow rectangle with spread and offset
        let shadow_rect = Rect::new(
            self.rect.x + self.offset_x - self.spread,
            self.rect.y + self.offset_y - self.spread,
            (self.rect.width as i32 + self.spread * 2) as u32,
            (self.rect.height as i32 + self.spread * 2) as u32
        );

        // For now, draw a simple blurred shadow approximation
        // Proper implementation would use Gaussian blur
        self.draw_simple_blur(shadow_rect);
    }

    fn draw_simple_blur(&mut self, shadow_rect: Rect) {
        // Draw multiple layers with decreasing opacity for blur effect
        let blur_steps = self.width.min(10);
        
        for i in 0..blur_steps {
            let expansion = (blur_steps - i) * self.width / blur_steps;
            let opacity_factor = 255 - ((i * 255) / blur_steps);
            let layer_opacity = Opacity::new((self.opacity.value() as u32 * opacity_factor as u32 / 255) as u8);
            
            let blur_rect = Rect::new(
                shadow_rect.x - expansion,
                shadow_rect.y - expansion,
                (shadow_rect.width as i32 + expansion * 2) as u32,
                (shadow_rect.height as i32 + expansion * 2) as u32
            );

            let desc = BlendDescriptor {
                mode: BlendMode::Normal,
                source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, layer_opacity) }),
                dest_rect: blur_rect,
                opacity: layer_opacity,
                mask: None,
            };
            self.layer.blend(&desc);
        }
    }
}

impl<'a> Layer<'a> {
    pub fn box_shadow(&'a mut self, rect: Rect) -> BoxShadow<'a> {
        BoxShadow::new(self, rect)
    }
}
