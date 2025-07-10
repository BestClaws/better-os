use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    prelude::*,
};

/// A lightweight, resizable draw target over a bit-packed framebuffer.
/// Used by windows and apps.
pub struct Canvas<'a> {
    buffer: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> Canvas<'a> {
    /// Create a new canvas over a framebuffer slice.
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        Self { buffer, width, height }
    }

    /// Clear canvas to black (off).
    pub fn clear(&mut self) {
        for byte in self.buffer.iter_mut() {
            *byte = 0;
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
        self.width = width;
        self.height = height;
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
    type Color = BinaryColor;
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

            // SSD1306 format: 1 byte = 8 vertical pixels
            let byte_index = x + (y / 8) * self.width as usize;
            let bit_index = y % 8;

            match color {
                BinaryColor::On => self.buffer[byte_index] |= 1 << bit_index,
                BinaryColor::Off => self.buffer[byte_index] &= !(1 << bit_index),
            }
        }

        Ok(())
    }
}
