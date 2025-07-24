use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{PixelColor, Rgb565},
    prelude::*,
};
use embedded_graphics_core::pixelcolor::raw::RawU16;

/// A lightweight, resizable draw target over an RGB565 framebuffer stored as u8 array.
/// Used by windows and app.
pub struct Canvas<'a> {
    buffer: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    /// Create a new canvas over a framebuffer slice.
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        assert!(buffer.len() >= (width * height * 2) as usize, "Buffer too small for RGB565");
        Self { buffer, width, height }
    }

    /// Clear canvas to black (off).
    pub fn clear(&mut self) {
        for chunk in self.buffer.chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = 0;
                chunk[1] = 0; // RGB565 black is 0x0000
            }
        }
    }

    /// Expose raw buffer.
    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }

    /// Expose mutable raw buffer.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer
    }

    /// Resize the canvas (does not reallocate, only updates metadata).
    pub fn resize(&mut self, width: u32, height: u32) {
        assert!(self.buffer.len() >= (width * height * 2) as usize, "Buffer too small for resized canvas");
        self.width = width;
        self.height = height;
    }

    /// Draw contents of another canvas into this canvas at offset.
    pub fn draw_from(&mut self, source: &Canvas, x_off: u32, y_off: u32) {
        let src_width = source.width.min(self.width.saturating_sub(x_off));
        let src_height = source.height.min(self.height.saturating_sub(y_off));

        for y in 0..src_height {
            for x in 0..src_width {
                let src_idx = ((x + y * source.width) * 2) as usize;
                let dst_idx = (((x + x_off) + (y + y_off) * self.width) * 2) as usize;

                if dst_idx + 1 < self.buffer.len() && src_idx + 1 < source.buffer.len() {
                    self.buffer[dst_idx] = source.buffer[src_idx];
                    self.buffer[dst_idx + 1] = source.buffer[src_idx + 1];
                }
            }
        }
    }

    /// Current width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Current height.
    pub fn height(&self) -> u32 {
        self.height
    }
}

impl OriginDimensions for Canvas<'_> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl DrawTarget for Canvas<'_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
                continue;
            }

            let x = x as usize;
            let y = y as usize;

            let index = (x + y * self.width as usize) * 2;
            if index + 1 < self.buffer.len() {
                let raw = color.into_storage(); // Get u16 in big-endian
                self.buffer[index] = (raw >> 8) as u8; // High byte
                self.buffer[index + 1] = raw as u8; // Low byte
            }
        }

        Ok(())
    }
}