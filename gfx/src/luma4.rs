//! Luma4 pixel format rasterizer (4-bit grayscale, 2 pixels per byte)

use core::{cmp::min, ptr};

use crate::colors::Color;
use crate::rasterizer::RasterTarget;
use math::udiv255;



/// Luma4 rasterizer that wraps a framebuffer
/// Each byte contains two 4-bit grayscale pixels (high nibble = even pixel, low nibble = odd pixel)
pub struct Luma4Rasterizer<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
}

impl<'a> Luma4Rasterizer<'a> {
    /// Create a new Luma4 rasterizer wrapping a framebuffer
    pub fn new(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self { buffer, width, height }
    }
}

impl<'a> RasterTarget for Luma4Rasterizer<'a> {
    #[inline]
    fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    fn height(&self) -> u16 {
        self.height
    }

}