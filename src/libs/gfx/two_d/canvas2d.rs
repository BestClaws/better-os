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
                // Sample gradient for each pixel
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
            }
        }
    }

}
