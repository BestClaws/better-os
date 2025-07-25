use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Gray4, GrayColor, PixelColor, Rgb565},
    prelude::*,
};
use embedded_graphics_core::pixelcolor::raw::RawU4;
use embedded_graphics_core::primitives::Rectangle;

/// A lightweight, resizable draw target over a framebuffer stored as a u8 array.
/// Supports multiple color formats via a generic type parameter.
pub struct Canvas<'a, C: PixelColor> {
    buffer: &'a mut [u8],
    width: u32,
    height: u32,
    _color: core::marker::PhantomData<C>,
}

impl<'a, C: PixelColor> Canvas<'a, C> {
    /// Create a new canvas over a framebuffer slice for a specific color type.
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        let bytes_per_pixel = if C::Raw::BITS_PER_PIXEL % 8 == 0 {
            C::Raw::BITS_PER_PIXEL / 8
        } else {
            (C::Raw::BITS_PER_PIXEL + 7) / 8 // Round up for partial bytes
        };
        assert!(
            buffer.len() >= (width as usize * height as usize * bytes_per_pixel as usize),
            "Buffer too small for color format"
        );
        Self {
            buffer,
            width,
            height,
            _color: core::marker::PhantomData,
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
        let bytes_per_pixel = if C::Raw::BITS_PER_PIXEL % 8 == 0 {
            C::Raw::BITS_PER_PIXEL / 8
        } else {
            (C::Raw::BITS_PER_PIXEL + 7) / 8 // Round up for partial bytes
        };
        assert!(
            self.buffer.len() >= (width as usize * height as usize * bytes_per_pixel as usize),
            "Buffer too small for resized canvas"
        );
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

impl<C: PixelColor> OriginDimensions for Canvas<'_, C> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

// DrawTarget implementation for Rgb565
impl DrawTarget for Canvas<'_, Rgb565> {
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
                let raw = color.into_storage();
                self.buffer[index] = (raw >> 8) as u8; // High byte
                self.buffer[index + 1] = raw as u8; // Low byte
            }
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let mut colors = colors.into_iter();
        let bottom_right = area.bottom_right().unwrap(); // Safe due to is_zero_sized check
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                if let Some(color) = colors.next() {
                    let index = ((x as u32 + y as u32 * self.width) * 2) as usize;
                    if index + 1 < self.buffer.len() {
                        let raw = color.into_storage();
                        self.buffer[index] = (raw >> 8) as u8;
                        self.buffer[index + 1] = raw as u8;
                    }
                }
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let raw = color.into_storage();
        let high_byte = (raw >> 8) as u8;
        let low_byte = raw as u8;

        let bottom_right = area.bottom_right().unwrap(); // Safe due to is_zero_sized check
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                let index = ((x as u32 + y as u32 * self.width) * 2) as usize;
                if index + 1 < self.buffer.len() {
                    self.buffer[index] = high_byte;
                    self.buffer[index + 1] = low_byte;
                }
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let raw = color.into_storage();
        let high_byte = (raw >> 8) as u8;
        let low_byte = raw as u8;

        for chunk in self.buffer.chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = high_byte;
                chunk[1] = low_byte;
            }
        }

        Ok(())
    }
}

// DrawTarget implementation for Gray4
impl DrawTarget for Canvas<'_, Gray4> {
    type Color = Gray4;
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

            let pixel_idx = x + y * self.width as usize;
            let byte_idx = pixel_idx / 2;
            let is_high_nibble = (pixel_idx % 2) == 0;

            if byte_idx < self.buffer.len() {
                let raw: RawU4 = color.into();
                let value = raw.into_inner();
                if is_high_nibble {
                    self.buffer[byte_idx] = (self.buffer[byte_idx] & 0x0F) | (value << 4);
                } else {
                    self.buffer[byte_idx] = (self.buffer[byte_idx] & 0xF0) | value;
                }
            }
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let mut colors = colors.into_iter();
        let bottom_right = area.bottom_right().unwrap(); // Safe due to is_zero_sized check
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                if let Some(color) = colors.next() {
                    let pixel_idx = (x as u32 + y as u32 * self.width) as usize;
                    let byte_idx = pixel_idx / 2;
                    let is_high_nibble = (pixel_idx % 2) == 0;

                    if byte_idx < self.buffer.len() {
                        let raw: RawU4 = color.into();
                        let value = raw.into_inner();
                        if is_high_nibble {
                            self.buffer[byte_idx] = (self.buffer[byte_idx] & 0x0F) | (value << 4);
                        } else {
                            self.buffer[byte_idx] = (self.buffer[byte_idx] & 0xF0) | value;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let raw: RawU4 = color.into();
        let value = raw.into_inner();
        let byte_value = (value << 4) | value; // Same color in both nibbles

        let bottom_right = area.bottom_right().unwrap(); // Safe due to is_zero_sized check
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                let pixel_idx = (x as u32 + y as u32 * self.width) as usize;
                let byte_idx = pixel_idx / 2;
                let is_high_nibble = (pixel_idx % 2) == 0;

                if byte_idx < self.buffer.len() {
                    if is_high_nibble {
                        self.buffer[byte_idx] = (self.buffer[byte_idx] & 0x0F) | (value << 4);
                    } else {
                        self.buffer[byte_idx] = (self.buffer[byte_idx] & 0xF0) | value;
                    }
                }
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let raw: RawU4 = color.into();
        let value = raw.into_inner();
        let byte_value = (value << 4) | value; // Same color in both nibbles

        for byte in self.buffer.iter_mut() {
            *byte = byte_value;
        }

        Ok(())
    }
}