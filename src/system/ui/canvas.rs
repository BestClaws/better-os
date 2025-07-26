use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Gray4, PixelColor, Rgb565},
    prelude::*,
    primitives::Rectangle,
};
use embedded_graphics_core::pixelcolor::raw::RawU4;
use core::marker::PhantomData;
use defmt::info;

/// A statically safe framebuffer-backed canvas.
pub struct Canvas<'a, C: PixelColor> {
    buffer: &'a mut [u8],
    width: u32,
    height: u32,
    _color: PhantomData<C>,
}

// ---------- Common for all Canvas types ----------
impl<'a, C: PixelColor> Canvas<'a, C> {
    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

impl<C: PixelColor> OriginDimensions for Canvas<'_, C> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

// ---------- Rgb565 ----------
impl<'a> Canvas<'a, Rgb565> {
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        let required = (width * height * 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Rgb565");

        Self {
            buffer,
            width,
            height,
            _color: PhantomData,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let required = (width * height * 2) as usize;
        assert!(self.buffer.len() >= required, "Buffer too small for Rgb565");
        self.width = width;
        self.height = height;
    }
}

impl DrawTarget for Canvas<'_, Rgb565> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
                continue;
            }
            let idx = ((x as u32 + y as u32 * self.width) * 2) as usize;
            let raw = color.into_storage();
            self.buffer[idx] = (raw >> 8) as u8;
            self.buffer[idx + 1] = raw as u8;
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Self::Color>,
    {
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let mut colors = colors.into_iter();
        let bottom_right = area.bottom_right().unwrap();
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                if let Some(color) = colors.next() {
                    let idx = ((x as u32 + y as u32 * self.width) * 2) as usize;
                    let raw = color.into_storage();
                    self.buffer[idx] = (raw >> 8) as u8;
                    self.buffer[idx + 1] = raw as u8;
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
        let high = (raw >> 8) as u8;
        let low = raw as u8;

        let bottom_right = area.bottom_right().unwrap();
        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                let idx = ((x as u32 + y as u32 * self.width) * 2) as usize;
                self.buffer[idx] = high;
                self.buffer[idx + 1] = low;
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {

        let raw = color.into_storage();
        let high = (raw >> 8) as u8;
        let low = raw as u8;

        for chunk in self.buffer.chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = high;
                chunk[1] = low;
            }
        }

        Ok(())
    }
}

// ---------- Gray4 ----------
impl<'a> Canvas<'a, Gray4> {
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        let required = ((width * height + 1) / 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Gray4");

        Self {
            buffer,
            width,
            height,
            _color: PhantomData,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let required = ((width * height + 1) / 2) as usize;
        assert!(self.buffer.len() >= required, "Buffer too small for Gray4");
        self.width = width;
        self.height = height;
    }
}

impl DrawTarget for Canvas<'_, Gray4> {
    type Color = Gray4;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        info!("draw_iter");
        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
                continue;
            }

            let pixel_idx = x as usize + y as usize * self.width as usize;
            let byte_idx = pixel_idx / 2;
            let is_high = pixel_idx % 2 == 0;

            let value = RawU4::from(color).into_inner();
            let byte = &mut self.buffer[byte_idx];
            if is_high {
                *byte = (*byte & 0x0F) | (value << 4);
            } else {
                *byte = (*byte & 0xF0) | value;
            }

        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Self::Color>,
    {
        info!("fill_contiguous");
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let mut colors = colors.into_iter();
        let bottom_right = area.bottom_right().unwrap();

        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                if let Some(color) = colors.next() {
                    let pixel_idx = x as usize + y as usize * self.width as usize;
                    let byte_idx = pixel_idx / 2;
                    let is_high = pixel_idx % 2 == 0;
                    let value = RawU4::from(color).into_inner();
                    let byte = &mut self.buffer[byte_idx];
                    if is_high {
                        *byte = (*byte & 0x0F) | (value << 4);
                    } else {
                        *byte = (*byte & 0xF0) | value;
                    }
                }
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Compute intersection and check for empty area
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            defmt::info!("Empty area, skipping fill");
            return Ok(());
        }

        // Log area coordinates as integers
        let top_left_x = area.top_left.x;
        let top_left_y = area.top_left.y;
        let bottom_right = area.bottom_right().unwrap();
        let bottom_right_x = bottom_right.x;
        let bottom_right_y = bottom_right.y;
        defmt::info!("Fill area: top_left=({}, {}), bottom_right=({}, {})", top_left_x, top_left_y, bottom_right_x, bottom_right_y);

        // Check buffer size (log but don't fail, as error is infallible)
        let required_bytes = (self.width as usize * self.height as usize + 1) / 2;
        if self.buffer.len() < required_bytes {
            defmt::info!("Buffer too small: {} < {}", self.buffer.len(), required_bytes);
            return Ok(());
        }

        // Get 4-bit color value
        let value = RawU4::from(color).into_inner() & 0x0F;
        defmt::info!("Color value: {}", value);

        // Optimize for full-byte fills
        let full_byte = (value << 4) | value; // Same value in both nibbles

        // Use inclusive range to handle single-pixel height
        for y in area.top_left.y..=bottom_right.y {
            let row_start = y as usize * self.width as usize;
            let mut x = area.top_left.x;

            // Handle unaligned start
            while x < bottom_right.x && (row_start + x as usize) % 2 != 0 {
                let pixel_idx = row_start + x as usize;
                let byte_idx = pixel_idx / 2;
                self.buffer[byte_idx] = (self.buffer[byte_idx] & 0xF0) | value; // Low nibble
                x += 1;
            }

            // Fill full bytes
            while x + 1 < bottom_right.x {
                let pixel_idx = row_start + x as usize;
                let byte_idx = pixel_idx / 2;
                self.buffer[byte_idx] = full_byte;
                x += 2;
            }

            // Handle unaligned end
            if x < bottom_right.x {
                let pixel_idx = row_start + x as usize;
                let byte_idx = pixel_idx / 2;
                self.buffer[byte_idx] = (self.buffer[byte_idx] & 0x0F) | (value << 4); // High nibble
            }
        }

        // Log buffer sample around the filled area
        let start_pixel = top_left_y as usize * self.width as usize + top_left_x as usize;
        let start_byte = start_pixel / 2;
        let sample_len = if self.buffer.len() > start_byte + 10 { 10 } else { self.buffer.len() - start_byte };
        defmt::info!("Buffer sample at byte {}: {:x}", start_byte, &self.buffer[start_byte..start_byte + sample_len]);

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        info!("clear");
        let value = RawU4::from(color).into_inner();
        let packed = (value << 4) | value;

        for byte in self.buffer.iter_mut() {
            *byte = packed;
        }

        Ok(())
    }
}

// ---------- Trait for generic creation ----------
pub trait PixelColorExt: PixelColor {
    fn new_canvas(buffer: &mut [u8], width: u32, height: u32) -> Canvas<Self>;
}

impl PixelColorExt for Rgb565 {
    fn new_canvas(buffer: &mut [u8], width: u32, height: u32) -> Canvas<Self> {
        Canvas::<Rgb565>::new(buffer, width, height)
    }
}

impl PixelColorExt for Gray4 {
    fn new_canvas(buffer: &mut [u8], width: u32, height: u32) -> Canvas<Self> {
        Canvas::<Gray4>::new(buffer, width, height)
    }
}
