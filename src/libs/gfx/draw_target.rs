/// DrawTarget trait - abstraction for any rendering surface
/// 
/// Based on LVGL's draw target model, this trait enables format-agnostic rendering.
/// Any surface that can provide a buffer, dimensions, and color format can implement this.

use super::core::{BlendDescriptor, Color, ColorFormat};

/// A surface that can be drawn to (display, offscreen buffer, texture)
pub trait DrawTarget {
    /// Get the color format of this target
    fn color_format(&self) -> ColorFormat;
    
    /// Get width and height in pixels
    fn dimensions(&self) -> (u32, u32);
    
    /// Get stride in bytes (may differ from width due to alignment)
    /// Default implementation assumes no padding
    fn stride(&self) -> usize {
        let (width, _) = self.dimensions();
        width as usize * self.color_format().bytes_per_pixel()
    }
    
    /// Mutable access to pixel buffer
    fn buffer_mut(&mut self) -> &mut [u8];
    
    /// Immutable access to pixel buffer
    fn buffer(&self) -> &[u8];
    
    /// Execute a blend operation
    /// This is the core rendering primitive - all drawing goes through this
    fn blend(&mut self, desc: &BlendDescriptor);
    
    /// Clear entire surface to a color
    fn clear(&mut self, color: Color);
    
    /// Get width
    fn width(&self) -> u32 {
        self.dimensions().0
    }
    
    /// Get height
    fn height(&self) -> u32 {
        self.dimensions().1
    }
}

/// Simple offscreen buffer implementing DrawTarget
pub struct OffscreenBuffer {
    buffer: alloc::vec::Vec<u8>,
    width: u32,
    height: u32,
    format: ColorFormat,
}

impl OffscreenBuffer {
    /// Create a new offscreen buffer
    pub fn new(width: u32, height: u32, format: ColorFormat) -> Self {
        let size = width as usize * height as usize * format.bytes_per_pixel();
        Self {
            buffer: alloc::vec![0; size],
            width,
            height,
            format,
        }
    }
    
    /// Get raw buffer
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
}

impl DrawTarget for OffscreenBuffer {
    fn color_format(&self) -> ColorFormat {
        self.format
    }
    
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
    
    fn buffer(&self) -> &[u8] {
        &self.buffer
    }
    
    fn blend(&mut self, _desc: &BlendDescriptor) {
        // TODO: Implement generic blending
        unimplemented!("Generic blending not yet implemented")
    }
    
    fn clear(&mut self, color: Color) {
        // Simple clear - format-specific optimizations can be added later
        match self.format {
            ColorFormat::Rgb565 => {
                let rgb565 = color.to_rgb565();
                for chunk in self.buffer.chunks_exact_mut(2) {
                    chunk[0] = (rgb565 >> 8) as u8;
                    chunk[1] = rgb565 as u8;
                }
            }
            ColorFormat::Rgb888 => {
                for chunk in self.buffer.chunks_exact_mut(3) {
                    chunk[0] = color.r;
                    chunk[1] = color.g;
                    chunk[2] = color.b;
                }
            }
            ColorFormat::Argb8888 | ColorFormat::Xrgb8888 => {
                for chunk in self.buffer.chunks_exact_mut(4) {
                    chunk[0] = 0xFF; // Alpha or unused
                    chunk[1] = color.r;
                    chunk[2] = color.g;
                    chunk[3] = color.b;
                }
            }
            ColorFormat::L8 => {
                let gray = color.to_grayscale().r;
                self.buffer.fill(gray);
            }
            ColorFormat::A8 => {
                self.buffer.fill(0xFF);
            }
        }
    }
}

extern crate alloc;
