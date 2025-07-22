use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::PixelColor,
    prelude::*,
};
use embedded_graphics_core::pixelcolor::raw::RawU8;

/// Custom RGB332 color type (3 bits red, 3 bits green, 2 bits blue).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Rgb332(u8);

impl Rgb332 {
    /// Create a new RGB332 color from raw R, G, B components.
    /// R and G are 3-bit (0-7), B is 2-bit (0-3).
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        // Mask and shift to fit RRRGGGBB format
        let r = (r & 0b111) << 5;
        let g = (g & 0b111) << 2;
        let b = b & 0b11;
        Rgb332(r | g | b)
    }

    /// Get raw u8 value.
    pub fn into_storage(self) -> u8 {
        self.0
    }
}

impl PixelColor for Rgb332 {
    type Raw = RawU8;

}

/// A lightweight, resizable draw target over an RGB332 framebuffer.
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
            *byte = 0; // RGB332 black is 0x00
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
                let src_idx = (x + y * source.width) as usize;
                let dst_idx = ((x + x_off) + (y + y_off) * self.width) as usize;

                if dst_idx < self.buffer.len() {
                    self.buffer[dst_idx] = source.buffer[src_idx];
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
    type Color = Rgb332;
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

            let index = x + y * self.width as usize;
            if index < self.buffer.len() {
                self.buffer[index] = color.into_storage();
            }
        }

        Ok(())
    }
}