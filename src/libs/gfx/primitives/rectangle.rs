/// Rectangle fill primitive
/// 
/// Draws a filled rectangle with optional rounded corners.

use super::super::core::{Color, ColorAlpha, Opacity, Rect};
use super::super::layer::Layer;

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
    pub fn draw(mut self) {
        // Basic implementation - just fill solid rect for now
        // TODO: Implement rounded corners, gradients, etc.
        
        let color_alpha = self.color.with_alpha(self.opacity.value());
        
        // Simple rect fill - no rounding yet
        let draw_rect = self.rect;
        self.fill_rect_simple(self.rect, color_alpha);
        
        // Mark the drawn region as dirty so compositor knows to flush it
        // This is a workaround - ideally Layer would track dirty regions
        // For now, we'll mark the whole screen dirty after any draw
        // TODO: Implement proper dirty region tracking in Layer
    }
    
    /// Simple rectangle fill (no rounding)
    fn fill_rect_simple(&mut self, rect: Rect, color: ColorAlpha) {
        use super::super::core::ColorFormat;
        
        // Clip to layer bounds
        let clip = self.layer.clip_area();
        let Some(clipped) = rect.intersection(clip) else {
            return; // Completely clipped out
        };
        
        // Get format-specific pixel operation
        match self.layer.color_format() {
            ColorFormat::Rgb565 => self.fill_rect_rgb565(clipped, color),
            _ => {
                // Other formats not yet implemented - skip silently
                // TODO: Implement fill for other formats
            }
        }
    }
    
    /// Fill rectangle in RGB565 format - optimized following LVGL approach
    fn fill_rect_rgb565(&mut self, rect: Rect, color: ColorAlpha) {
        use super::super::blend_rgb565;
        use super::super::core::Point;
        
        // Convert color to RGB565
        let rgb565 = color.to_color().to_rgb565();
        let alpha = color.a;
        
        let layer_width = self.layer.width();
        let buf_area = self.layer.buf_area();
        
        // Calculate buffer coordinates for top-left corner (ONCE, not per-pixel)
        let Some(buf_start) = self.layer.screen_to_buffer(Point::new(rect.x, rect.y)) else {
            return; // Completely outside buffer area
        };
        
        if buf_start.x < 0 || buf_start.y < 0 {
            return;
        }
        
        let buf_x = buf_start.x as u32;
        let buf_y = buf_start.y as u32;
        
        // Clamp dimensions to buffer bounds
        let max_width = (layer_width - buf_x).min(rect.width);
        let max_height = (self.layer.height() - buf_y).min(rect.height);
        
        if max_width == 0 || max_height == 0 {
            return;
        }
        
        let buffer = self.layer.buffer_mut();
        
        // LVGL optimization: Separate loops for opaque vs transparent
        // This avoids branching in the hot inner loop
        
        if alpha == 255 {
            // FAST PATH: Opaque fill - no blending needed
            for y in 0..max_height {
                let row_offset = ((buf_y + y) * layer_width + buf_x) as usize * 2;
                
                // Fill this row with solid color
                for x in 0..max_width {
                    let idx = row_offset + x as usize * 2;
                    if idx + 1 < buffer.len() {
                        // Write RGB565 in big-endian (hi, lo) format
                        buffer[idx] = (rgb565 >> 8) as u8;
                        buffer[idx + 1] = rgb565 as u8;
                    }
                }
            }
        } else if alpha > 0 {
            // SLOW PATH: Alpha blending required
            for y in 0..max_height {
                let row_offset = ((buf_y + y) * layer_width + buf_x) as usize * 2;
                
                for x in 0..max_width {
                    let idx = row_offset + x as usize * 2;
                    if idx + 1 < buffer.len() {
                        // Read background in big-endian format
                        let bg = ((buffer[idx] as u16) << 8) | (buffer[idx + 1] as u16);
                        // Blend
                        let blended = blend_rgb565(bg, rgb565, alpha);
                        // Write back
                        buffer[idx] = (blended >> 8) as u8;
                        buffer[idx + 1] = blended as u8;
                    }
                }
            }
        }
        // If alpha == 0, nothing to draw (fully transparent)
    }
}

/// Extension methods for Layer  
impl<'b> Layer<'b> {
    pub fn fill<'a>(&'a mut self, rect: Rect) -> Fill<'a, 'b> {
        Fill::new(self, rect)
    }
}
