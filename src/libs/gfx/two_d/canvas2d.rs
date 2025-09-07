use super::{Rasterizer, Rgb565, Rgba8888};
use crate::libs::gfx::two_d::fluent::DrawTarget;

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

    /// Access the underlying rasterizer (RGB565-based) when needed.
    pub fn raster_mut(&mut self) -> &mut dyn Rasterizer { self.raster }

    /// Clear the target with an RGBA color (alpha ignored for full clear).
    pub fn clear_rgba(&mut self, color: Rgba8888) {
        let rgb565 = color.to_rgb565();
        self.raster.clear(rgb565);
    }

    /// Set pixel using RGBA color; uses alpha to blend over background.
    pub fn set_pixel_rgba(&mut self, x: i32, y: i32, color: Rgba8888) {
        let rgb = color.to_rgb565();
        if color.a == 255 { self.raster.set_pixel(x, y, rgb); }
        else if color.a == 0 { /* no-op */ }
        else { self.raster.blend_pixel(x, y, rgb, color.a); }
    }

    /// Fill a rectangle with a solid RGBA color.
    pub fn fill_rect_rgba(&mut self, top_left_x: i32, top_left_y: i32, width: u32, height: u32, color: Rgba8888) {
        let rect = crate::libs::gfx::two_d::Rect::new(
            crate::libs::gfx::two_d::Point::new(top_left_x, top_left_y),
            crate::libs::gfx::two_d::Size::new(width, height)
        );
        let rgb = color.to_rgb565();
        if color.a == 255 {
            self.raster.set_pixels_rect(rect, rgb);
        } else if color.a > 0 {
            self.raster.blend_pixels_rect(rect, rgb, color.a);
        }
    }
}

impl DrawTarget for Canvas2D<'_> {
    fn size(&self) -> super::Size { super::Size::new(self.raster.width(), self.raster.height()) }
    fn clear(&mut self, color: Rgb565) { self.raster.clear(color) }
    fn raster_mut(&mut self) -> &mut dyn Rasterizer { self.raster }
}


