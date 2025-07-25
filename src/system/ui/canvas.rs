use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Gray4, PixelColor, Rgb565},
    prelude::*,
    primitives::Rectangle,
};
use embedded_graphics_core::pixelcolor::raw::RawU4;
use core::marker::PhantomData;

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
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Canvas<'a, Rgb565> {
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




// ---------- Gray4 ----------
impl<'a> Canvas<'a, Gray4> {
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Canvas<'a, Gray4> {
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
        let area = area.intersection(&Rectangle::new(Point::zero(), self.size()));
        if area.is_zero_sized() {
            return Ok(());
        }

        let value = RawU4::from(color).into_inner();
        let packed = (value << 4) | value;

        let bottom_right = area.bottom_right().unwrap();

        for y in area.top_left.y..bottom_right.y {
            for x in area.top_left.x..bottom_right.x {
                let pixel_idx = x as usize + y as usize * self.width as usize;
                let byte_idx = pixel_idx / 2;
                let is_high = pixel_idx % 2 == 0;
                let byte = &mut self.buffer[byte_idx];
                if is_high {
                    *byte = (*byte & 0x0F) | (value << 4);
                } else {
                    *byte = (*byte & 0xF0) | value;
                }
            }
        }

        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let value = RawU4::from(color).into_inner();
        let packed = (value << 4) | value;

        for byte in self.buffer.iter_mut() {
            *byte = packed;
        }

        Ok(())
    }
}
