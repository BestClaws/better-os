//! Luma4 pixel format rasterizer (4-bit grayscale, 2 pixels per byte)

use crate::colors::Color;
use crate::rasterizer::RasterTarget;

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

    /// Get the byte index and shift for a pixel coordinate
    #[inline]
    fn pixel_address(&self, x: u16, y: u16) -> (usize, u8) {
        let pixel_idx = y as usize * self.width as usize + x as usize;
        let byte_idx = pixel_idx / 2;
        let shift = if pixel_idx & 1 == 0 { 4 } else { 0 }; // Even pixels in high nibble, odd in low
        (byte_idx, shift)
    }

    /// Read a pixel value (0-15)
    #[inline]
    fn read_pixel(&self, x: u16, y: u16) -> u8 {
        let (byte_idx, shift) = self.pixel_address(x, y);
        (self.buffer[byte_idx] >> shift) & 0x0F
    }

    /// Write a pixel value (0-15)
    #[inline]
    fn write_pixel(&mut self, x: u16, y: u16, luma: u8) {
        let (byte_idx, shift) = self.pixel_address(x, y);
        let mask = 0x0F << shift;
        self.buffer[byte_idx] = (self.buffer[byte_idx] & !mask) | ((luma & 0x0F) << shift);
    }
}

impl<'a> RasterTarget for Luma4Rasterizer<'a> {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        let luma = color_to_luma4(color);
        
        for x in x_start..x_start.saturating_add(length) {
            if x < self.width && y < self.height {
                self.write_pixel(x, y, luma);
            }
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        let fg_luma = color_to_luma4(color);
        
        for (i, &alpha) in coverage.iter().enumerate() {
            let x = x_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.write_pixel(x, y, fg_luma);
                } else {
                    // Alpha blend
                    let bg_luma = self.read_pixel(x, y);
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    let out_luma = ((fg_luma as u16 * alpha_norm + bg_luma as u16 * inv_alpha) / 255) as u8;
                    self.write_pixel(x, y, out_luma & 0x0F);
                }
            }
        }
    }

    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        for (i, (&color, &alpha)) in colors.iter().zip(coverage.iter()).enumerate() {
            let x = x_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let fg_luma = color_to_luma4(color);
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.write_pixel(x, y, fg_luma);
                } else {
                    // Alpha blend
                    let bg_luma = self.read_pixel(x, y);
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    let out_luma = ((fg_luma as u16 * alpha_norm + bg_luma as u16 * inv_alpha) / 255) as u8;
                    self.write_pixel(x, y, out_luma & 0x0F);
                }
            }
        }
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        let luma = color_to_luma4(color);
        
        for y in y_start..y_start.saturating_add(length) {
            if x < self.width && y < self.height {
                self.write_pixel(x, y, luma);
            }
        }
    }

    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        let fg_luma = color_to_luma4(color);
        
        for (i, &alpha) in coverage.iter().enumerate() {
            let y = y_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.write_pixel(x, y, fg_luma);
                } else {
                    // Alpha blend
                    let bg_luma = self.read_pixel(x, y);
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    let out_luma = ((fg_luma as u16 * alpha_norm + bg_luma as u16 * inv_alpha) / 255) as u8;
                    self.write_pixel(x, y, out_luma & 0x0F);
                }
            }
        }
    }

    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        for (i, (&color, &alpha)) in colors.iter().zip(coverage.iter()).enumerate() {
            let y = y_start + i as u16;
            if x < self.width && y < self.height && alpha > 0 {
                let fg_luma = color_to_luma4(color);
                
                if alpha == 255 {
                    // Fast path: fully opaque
                    self.write_pixel(x, y, fg_luma);
                } else {
                    // Alpha blend
                    let bg_luma = self.read_pixel(x, y);
                    let alpha_norm = alpha as u16;
                    let inv_alpha = 255 - alpha_norm;
                    let out_luma = ((fg_luma as u16 * alpha_norm + bg_luma as u16 * inv_alpha) / 255) as u8;
                    self.write_pixel(x, y, out_luma & 0x0F);
                }
            }
        }
    }

    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        let luma = color_to_luma4(color);
        
        for dy in 0..height {
            let py = y.saturating_add(dy);
            if py >= self.height {
                break;
            }
            
            for dx in 0..width {
                let px = x.saturating_add(dx);
                if px >= self.width {
                    break;
                }
                
                self.write_pixel(px, py, luma);
            }
        }
    }
}

/// Convert RGB color to 4-bit grayscale using ITU-R BT.601 luma coefficients
/// Returns a value in the range 0-15
#[inline]
fn color_to_luma4(color: Color) -> u8 {
    // Y = 0.299*R + 0.587*G + 0.114*B
    // Use integer approximation: Y = (77*R + 150*G + 29*B) / 256
    let luma8 = ((77 * color.r() as u16 + 150 * color.g() as u16 + 29 * color.b() as u16) / 256) as u8;
    // Convert 8-bit (0-255) to 4-bit (0-15)
    luma8 >> 4
}
