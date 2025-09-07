use super::{Rasterizer, Rgb565, Rgba8888};
use super::types::CanvasColor;

/// Canvas2D shim that accepts RGBA colors and forwards to an underlying Rasterizer.
/// No backing buffer; conversions happen on write to the target rasterizer.
pub struct Canvas2D<'a, C: CanvasColor = Rgba8888> {
    raster: &'a mut dyn Rasterizer,
    _marker: core::marker::PhantomData<C>,
}

impl<'a, C: CanvasColor> Canvas2D<'a, C> {
    pub fn new(raster: &'a mut dyn Rasterizer) -> Self {
        Self { raster, _marker: core::marker::PhantomData }
    }

    #[inline(always)]
    pub fn width(&self) -> u32 { self.raster.width() }
    #[inline(always)]
    pub fn height(&self) -> u32 { self.raster.height() }

    /// Access the underlying rasterizer (RGB565-based) when needed.
    pub fn raster_mut(&mut self) -> &mut dyn Rasterizer { self.raster }

    /// Clear the target with an RGBA color (alpha ignored for full clear).
    pub fn clear_color(&mut self, color: C) {
        let rgb565 = color.to_rgb565();
        self.raster.clear(rgb565);
    }

    /// Set pixel using RGBA color; uses alpha to blend over background.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: C) {
        let rgb = color.to_rgb565();
        let a = color.alpha_u8();
        if a == 255 { self.raster.set_pixel(x, y, rgb); }
        else if a == 0 { /* no-op */ }
        else { self.raster.blend_pixel(x, y, rgb, a); }
    }

    /// Fill a rectangle with a solid RGBA color.
    pub fn fill_rect(&mut self, top_left_x: i32, top_left_y: i32, width: u32, height: u32, color: C) {
        let rect = crate::libs::gfx::two_d::Rect::new(
            crate::libs::gfx::two_d::Point::new(top_left_x, top_left_y),
            crate::libs::gfx::two_d::Size::new(width, height)
        );
        let rgb = color.to_rgb565();
        let a = color.alpha_u8();
        if a == 255 {
            self.raster.set_pixels_rect(rect, rgb);
        } else if a > 0 {
            self.raster.blend_pixels_rect(rect, rgb, a);
        }
    }
}
