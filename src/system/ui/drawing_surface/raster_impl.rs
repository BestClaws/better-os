//! Rasterizer trait implementation for `DrawingSurface`.
//!
//! This connects the format-agnostic 2D algorithms to the storage format
//! via `PixelOps`. All inputs/outputs are in `Rgba8888`; storage is in
//! the negotiated `PixelFormat` (Rgb565 reference implementation).
use crate::libs::gfx::two_d::{Rasterizer, Rgba8888};
use crate::util::math::primitives::Rect;
use super::surface::DrawingSurface;

impl Rasterizer for DrawingSurface<'_> {
    fn width(&self) -> u32 { self.width() }
    fn height(&self) -> u32 { self.height() }

    fn set_pixel(&mut self, x: i32, y: i32, color: Rgba8888) {
        self.set_pixel_internal(x, y, color);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        self.blend_pixel_internal(x, y, color, coverage);
    }

    fn set_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: Rgba8888) {
        self.set_pixels_horizontal_internal(x, y, width, color);
    }

    fn set_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: Rgba8888) {
        self.set_pixels_vertical_internal(x, y, height, color);
    }

    fn set_pixels_rect(&mut self, rect: Rect, color: Rgba8888) {
        self.set_pixels_rect_internal(rect, color);
    }

    fn get_pixel(&self, x: i32, y: i32) -> Rgba8888 {
        self.get_pixel_internal(x, y)
    }
}


