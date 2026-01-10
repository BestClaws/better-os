/// Rectangle fill primitive
/// 
/// Draws a filled rectangle with optional rounded corners.

use super::super::core::{blend::{BlendDescriptor, BlendMode, BlendSource}, Color, ColorAlpha, Opacity, Rect};
use super::super::layer::Layer;
use super::super::draw_target::DrawTarget;

/// Filled rectangle primitive
pub struct Fill<'a, 'b> {
    layer: &'a mut Layer<'b>,
    rect: Rect,
    color: Color,
    opacity: Opacity,
    radius: i32,
}

impl<'a, 'b> Fill<'a, 'b> {
    /// Create a new fill primitive
    pub fn new(layer: &'a mut Layer<'b>, rect: Rect) -> Self {
        Self {
            layer,
            rect,
            color: Color::WHITE,
            opacity: Opacity::OPAQUE,
            radius: 0,
        }
    }
    
    /// Set fill color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    
    /// Set opacity (0 = transparent, 255 = opaque)
    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }
    
    /// Set corner radius (0 = sharp corners)
    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = radius.max(0);
        self
    }
    
    /// Make it a circle (radius = min(width, height) / 2)
    pub fn circle(mut self) -> Self {
        let min_dim = self.rect.width.min(self.rect.height) as i32;
        self.radius = min_dim / 2;
        self
    }
    
    /// Execute the draw operation
    pub fn draw(self) {
        // Clip to layer bounds
        let clip = self.layer.clip_area();
        let Some(clipped) = self.rect.intersection(clip) else {
            return; // Completely clipped out
        };
        
        // Create color with alpha
        let color_alpha = ColorAlpha::from_color(self.color, self.opacity.value());
        
        // Create blend descriptor
        let blend_desc = BlendDescriptor::solid_fill(
            clipped,
            color_alpha,
            BlendMode::Normal,
        );
        
        // Use DrawTarget::blend() - the proper LVGL architecture
        self.layer.blend(&blend_desc);
        
        // TODO: Add support for rounded corners using masks
    }
}

/// Extension methods for Layer  
impl<'b> Layer<'b> {
    pub fn fill<'a>(&'a mut self, rect: Rect) -> Fill<'a, 'b> {
        Fill::new(self, rect)
    }
}
