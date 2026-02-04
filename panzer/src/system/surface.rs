//! Surface abstraction for rendering
//!
//! A Surface wraps a frame buffer and provides RasterTarget implementation,
//! allowing applications to draw to windows without direct buffer access.

use gfx::colors::Color;
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::luma4::Luma4Rasterizer;

/// Pixel format for surfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb565,
    Luma4,
}

/// A drawing surface backed by a frame buffer
pub struct Surface<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
    format: PixelFormat,
}

impl<'a> Surface<'a> {
    /// Create a new RGB565 surface
    pub fn new_rgb565(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self {
            buffer,
            width,
            height,
            format: PixelFormat::Rgb565,
        }
    }

    /// Create a new LUMA4 surface
    pub fn new_luma4(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self {
            buffer,
            width,
            height,
            format: PixelFormat::Luma4,
        }
    }

    /// Get the pixel format
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// Get a rasterizer for this surface
    pub fn rasterizer(&mut self) -> SurfaceRasterizer<'_> {
        match self.format {
            PixelFormat::Rgb565 => {
                SurfaceRasterizer::Rgb565(Rgb565Rasterizer::new(self.buffer, self.width, self.height))
            }
            PixelFormat::Luma4 => {
                SurfaceRasterizer::Luma4(Luma4Rasterizer::new(self.buffer, self.width, self.height))
            }
        }
    }

    /// Clear the surface with a color
    pub fn clear(&mut self, color: Color) {
        let width = self.width;
        let height = self.height;
        match self.rasterizer() {
            SurfaceRasterizer::Rgb565(mut rast) => {
                rast.fill_solid_rect(0, 0, width, height, color);
            }
            SurfaceRasterizer::Luma4(mut rast) => {
                rast.fill_solid_rect(0, 0, width, height, color);
            }
        }
    }
}

/// Enum wrapping different rasterizer types
pub enum SurfaceRasterizer<'a> {
    Rgb565(Rgb565Rasterizer<'a>),
    Luma4(Luma4Rasterizer<'a>),
}

impl<'a> RasterTarget for SurfaceRasterizer<'a> {
    fn width(&self) -> u16 {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.width(),
            SurfaceRasterizer::Luma4(r) => r.width(),
        }
    }

    fn height(&self) -> u16 {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.height(),
            SurfaceRasterizer::Luma4(r) => r.height(),
        }
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_hspan(y, x_start, color, length),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_hspan(y, x_start, color, length),
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_solid_hspan(y, x_start, color, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_solid_hspan(y, x_start, color, coverage),
        }
    }

    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_color_hspan(y, x_start, colors, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_color_hspan(y, x_start, colors, coverage),
        }
    }

    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_rect(x, y, width, height, color),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_rect(x, y, width, height, color),
        }
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.fill_solid_vspan(x, y_start, color, length),
            SurfaceRasterizer::Luma4(r) => r.fill_solid_vspan(x, y_start, color, length),
        }
    }

    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_solid_vspan(x, y_start, color, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_solid_vspan(x, y_start, color, coverage),
        }
    }

    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        match self {
            SurfaceRasterizer::Rgb565(r) => r.blend_color_vspan(x, y_start, colors, coverage),
            SurfaceRasterizer::Luma4(r) => r.blend_color_vspan(x, y_start, colors, coverage),
        }
    }
}
