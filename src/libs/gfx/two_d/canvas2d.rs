use super::{Rasterizer, Rgba8888, Point, Size, Paint};

/// Canvas2D shim that accepts RGBA colors and forwards to an underlying Rasterizer.
/// No backing buffer; conversions happen on write to the target rasterizer.
pub struct Canvas2D<'a> {
    raster: &'a mut dyn Rasterizer,
}

impl<'a> Canvas2D<'a> {
    pub fn new(raster: &'a mut dyn Rasterizer) -> Self {
        Self { raster }
    }

    #[inline(always)]
    pub fn width(&self) -> u32 { self.raster.width() }
    #[inline(always)]
    pub fn height(&self) -> u32 { self.raster.height() }

    /// Access the underlying rasterizer when needed.
    pub fn raster_mut(&mut self) -> &mut dyn Rasterizer { self.raster }
    
    /// Fast path: Fill rectangle with solid color (no alpha blending)
    #[inline]
    pub fn fill_rect_fast(&mut self, rect: super::types::Rect, color: Rgba8888) {
        if color.a == 255 {
            self.raster.set_pixels_rect(rect, color);
        } else if color.a > 0 {
            self.raster.blend_pixels_rect(rect, color, color.a);
        }
    }
    
    /// Fast path: Fill horizontal line
    #[inline]
    pub fn fill_hline_fast(&mut self, x_start: i32, x_end: i32, y: i32, color: Rgba8888) {
        if color.a == 255 {
            self.raster.set_pixels_hline(x_start, x_end, y, color);
        } else if color.a > 0 {
            for x in x_start..=x_end {
                self.raster.blend_pixel(x, y, color, color.a);
            }
        }
    }

    /// Clear the target with an RGBA color (alpha ignored for full clear).
    pub fn clear_color(&mut self, color: Rgba8888) {
        self.raster.clear(color.with_alpha(255));
    }

    /// Set pixel using RGBA color; uses alpha to blend over background.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgba8888) {
        let a = color.a;
        if a == 255 { self.raster.set_pixel(x, y, color); }
        else if a == 0 { /* no-op */ }
        else { self.raster.blend_pixel(x, y, color, a); }
    }

    /// Fill a rectangle with a solid RGBA color.
    pub fn fill_rect(&mut self, top_left_x: i32, top_left_y: i32, width: u32, height: u32, color: Rgba8888) {
        let rect = crate::libs::gfx::two_d::types::Rect::new(
            Point::new(top_left_x, top_left_y),
            Size::new(width, height)
        );
        let a = color.a;
        if a == 255 {
            self.raster.set_pixels_rect(rect, color);
        } else if a > 0 {
            self.raster.blend_pixels_rect(rect, color, a);
        }
    }

    /// Fill a rectangle with paint (solid color or gradient).
    pub fn fill_rect_with_paint(&mut self, top_left_x: i32, top_left_y: i32, width: u32, height: u32, paint: &Paint) {
        let rect = crate::libs::gfx::two_d::types::Rect::new(
            Point::new(top_left_x, top_left_y),
            Size::new(width, height)
        );
        
        match paint {
            Paint::Solid(color) => {
                let a = color.a;
                if a == 255 {
                    self.raster.set_pixels_rect(rect, *color);
                } else if a > 0 {
                    self.raster.blend_pixels_rect(rect, *color, a);
                }
            }
            _ => {
                // Optimized gradient rendering with reduced sampling
                let width = rect.size.width as i32;
                let height = rect.size.height as i32;
                let area = width * height;
                
                // Use adaptive sampling based on area size
                let sample_step = if area > 50000 { 4 } else if area > 10000 { 2 } else { 1 };
                
                if sample_step == 1 {
                    // Full resolution for small areas
                    for y in rect.top_left.y..=rect.bottom() {
                        for x in rect.top_left.x..=rect.right() {
                            let color = paint.sample_at(Point::new(x, y));
                            if color.a == 255 {
                                self.raster.set_pixel(x, y, color);
                            } else if color.a > 0 {
                                self.raster.blend_pixel(x, y, color, color.a);
                            }
                        }
                    }
                } else {
                    // Reduced sampling with block filling for large areas
                    for y in (rect.top_left.y..=rect.bottom()).step_by(sample_step) {
                        for x in (rect.top_left.x..=rect.right()).step_by(sample_step) {
                            let color = paint.sample_at(Point::new(x, y));
                            if color.a > 0 {
                                // Fill sample_step x sample_step block
                                for dy in 0..sample_step {
                                    for dx in 0..sample_step {
                                        let px = x + dx as i32;
                                        let py = y + dy as i32;
                                        if px <= rect.right() && py <= rect.bottom() {
                                            if color.a == 255 {
                                                self.raster.set_pixel(px, py, color);
                                            } else {
                                                self.raster.blend_pixel(px, py, color, color.a);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

}
