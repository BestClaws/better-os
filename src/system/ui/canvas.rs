use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::BinaryColor,
    prelude::*,
};

/// A lightweight, resizable draw target over a bit-packed framebuffer.
/// Used by windows and app.
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

    /// Draw contents of another canvas into this canvas at offset.
    pub fn draw_from(&mut self, source: &Canvas, x_off: u32, y_off: u32) {
        let src_width = source.width.min(self.width.saturating_sub(x_off));
        let src_height = source.height.min(self.height.saturating_sub(y_off));

        for y in 0..src_height {
            for x in 0..src_width {
                let src_idx = x + (y / 8) * source.width;
                let dst_idx = (x + x_off) + ((y + y_off) / 8) * self.width;

                let bit = 1 << (y % 8);
                let src_byte = source.buffer[(src_idx) as usize];

                let pixel_on = src_byte & bit != 0;

                if dst_idx < (self.width * (self.height / 8)) {
                    if pixel_on {
                        self.buffer[dst_idx as usize] |= 1 << ((y + y_off) % 8);
                    } else {
                        self.buffer[dst_idx as usize] &= !(1 << ((y + y_off) % 8));
                    }
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
