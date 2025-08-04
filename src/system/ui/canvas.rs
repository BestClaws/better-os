use core::marker::PhantomData;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Gray4, PixelColor, Rgb565},
    prelude::*,
    primitives::Rectangle,
};
use embedded_graphics_core::pixelcolor::raw::RawU4;

/// A statically-safe, framebuffer-backed drawing canvas.
///
/// Supports dirty region tracking and pixel-level rendering. Efficient
/// for embedded systems that rely on partial screen updates.
///
/// Supported color formats: `Rgb565`, `Gray4`.
pub struct Canvas<'a, C: PixelColor> {
    buf: Option<&'a mut [u8]>,
    // buffer: &'a mut [u8],
    width: u32,
    height: u32,
    _color: PhantomData<C>,
    dirty_region: Option<Rectangle>,
}

// ===== Common (Generic) Canvas Implementation =====
impl<'a, C: PixelColor> Canvas<'a, C> {

    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: None,
            width,
            height,
            _color: PhantomData,
            dirty_region: Some(Rectangle::new(Point::zero(), Size::new(width, height))),
        }
    }
    
    pub(crate) fn relinquish(&mut self) {
        self.buf = None;
    }

    fn _buf(&self) -> &[u8] {
        if let Some(buf) = &self.buf {
            // return
            buf
        } else {
            panic!("Buffer not initialized");
        }
    }

    fn _buf_mut(&mut self) -> &mut [u8] {
        if let Some(buf) = &mut self.buf {
            // return
            buf
        } else {
            panic!("Buffer not initialized");
        }
    }
    /// Immutable access to the raw pixel buffer.
    pub fn buffer(&self) -> &[u8] {
        self._buf()
    }

    /// Mutable access to the raw pixel buffer.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self._buf_mut()
    }

    /// Returns the canvas width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the canvas height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Clears the dirty region, marking the canvas as fully flushed.
    pub fn flush(&mut self) {
        self.dirty_region = None;
    }

    /// Returns the current dirty region (if any).
    pub fn dirty_region(&self) -> Option<Rectangle> {
        self.dirty_region
    }

    /// Expands the dirty region to include the given rectangle.
    fn update_dirty(&mut self, new: Rectangle) {
        self.dirty_region = Some(match self.dirty_region {
            Some(existing) => union_rect(existing, new),
            None => new,
        });
    }
    
    
    
}

impl<C: PixelColor> OriginDimensions for Canvas<'_, C> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

// ===== Rgb565 Implementation =====
impl<'a> Canvas<'a, Rgb565> {


    /// Resizes the canvas in-place, validating the buffer size.
    pub fn resize(&mut self, width: u32, height: u32) {
        let required = (width * height * 2) as usize;
        assert!(self._buf_mut().len() >= required, "Buffer too small for Rgb565");

        self.width = width;
        self.height = height;
    }

    pub fn materialize(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width * self.height * 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Rgb565");
        self.buf = Some(buffer);
    }
}

impl DrawTarget for Canvas<'_, Rgb565> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let mut region = None;

        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
                continue;
            }

            let offset = ((x as u32 + y as u32 * self.width) * 2) as usize;
            let raw = color.into_storage();
            self._buf_mut()[offset] = (raw >> 8) as u8;
            self._buf_mut()[offset + 1] = raw as u8;

            let point = Point::new(x, y);
            region = Some(region.map_or(Rectangle::new(point, Size::new(1, 1)), |r| union_rect(r, Rectangle::new(point, Size::new(1, 1)))));
        }

        if let Some(r) = region {
            self.update_dirty(r);
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Self::Color>,
    {
        let (x0, y0, x1, y1) = clip_rect(area, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let mut iter = colors.into_iter();
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(color) = iter.next() {
                    let idx = (x + y * self.width) * 2;
                    let raw = color.into_storage();
                    self._buf_mut()[idx as usize] = (raw >> 8) as u8;
                    self._buf_mut()[idx as usize + 1] = raw as u8;
                } else {
                    return Ok(());
                }
            }
        }

        self.update_dirty(Rectangle::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0)));
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let (x0, y0, x1, y1) = clip_rect(area, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;

        for y in y0..y1 {
            let row_start = (x0 + y * self.width) * 2;
            let row_end = (x1 + y * self.width) * 2;
            for idx in (row_start as usize..row_end as usize).step_by(2) {
                self._buf_mut()[idx] = hi;
                self._buf_mut()[idx + 1] = lo;
            }
        }

        self.update_dirty(Rectangle::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0)));
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;

        for chunk in self._buf_mut().chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = hi;
                chunk[1] = lo;
            }
        }

        self.update_dirty(Rectangle::new(Point::zero(), Size::new(self.width, self.height)));
        Ok(())
    }
}

// ===== Gray4 Implementation =====
impl<'a> Canvas<'a, Gray4> {
    pub fn resize(&mut self, width: u32, height: u32) {
        let required = ((width * height + 1) / 2) as usize;
        assert!(self._buf_mut().len() >= required, "Buffer too small for Gray4");

        self.width = width;
        self.height = height;
    }

    pub(crate) fn set_resources(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width * self.height / 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Gray4");
        self.buf = Some(buffer);
    }
}

impl DrawTarget for Canvas<'_, Gray4> {
    type Color = Gray4;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let mut region = None;

        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
                continue;
            }

            let index = x as usize + y as usize * self.width as usize;
            let byte_index = index / 2;
            let high = index % 2 == 0;

            let val = RawU4::from(color).into_inner();
            let byte = &mut self._buf_mut()[byte_index];

            *byte = if high {
                (*byte & 0x0F) | (val << 4)
            } else {
                (*byte & 0xF0) | val
            };

            let point = Point::new(x, y);
            region = Some(region.map_or(Rectangle::new(point, Size::new(1, 1)), |r| union_rect(r, Rectangle::new(point, Size::new(1, 1)))));
        }

        if let Some(r) = region {
            self.update_dirty(r);
        }

        Ok(())
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Self::Color>,
    {
        let (x0, y0, x1, y1) = clip_rect(area, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let mut iter = colors.into_iter();
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(color) = iter.next() {
                    let idx = x as usize + y as usize * self.width as usize;
                    let byte_idx = idx / 2;
                    let high = idx % 2 == 0;
                    let val = RawU4::from(color).into_inner();
                    let byte = &mut self._buf_mut()[byte_idx];

                    *byte = if high {
                        (*byte & 0x0F) | (val << 4)
                    } else {
                        (*byte & 0xF0) | val
                    };
                } else {
                    return Ok(());
                }
            }
        }

        self.update_dirty(Rectangle::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0)));
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let (x0, y0, x1, y1) = clip_rect(area, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return Ok(());
        }

        let val = RawU4::from(color).into_inner();
        let packed = (val << 4) | val;

        for y in y0..y1 {
            let start_idx = (y * self.width + x0) as usize;
            let end_idx = (y * self.width + x1) as usize;

            for pixel in start_idx..end_idx {
                let byte_idx = pixel / 2;
                let high = pixel % 2 == 0;
                let byte = &mut self._buf_mut()[byte_idx];

                *byte = if high {
                    (*byte & 0x0F) | (val << 4)
                } else {
                    (*byte & 0xF0) | val
                };
            }
        }

        self.update_dirty(Rectangle::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0)));
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let val = RawU4::from(color).into_inner();
        let packed = (val << 4) | val;
        self._buf_mut().fill(packed);

        self.update_dirty(Rectangle::new(Point::zero(), Size::new(self.width, self.height)));
        Ok(())
    }




}

// ===== Helpers =====

/// Clips the given rectangle to canvas bounds and returns coordinates.
fn clip_rect(area: &Rectangle, width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x0 = area.top_left.x.max(0) as u32;
    let y0 = area.top_left.y.max(0) as u32;
    let x1 = (area.top_left.x as u32 + area.size.width).min(width);
    let y1 = (area.top_left.y as u32 + area.size.height).min(height);
    (x0, y0, x1, y1)
}

/// Computes the union (bounding box) of two rectangles.
fn union_rect(r1: Rectangle, r2: Rectangle) -> Rectangle {
    let left = r1.top_left.x.min(r2.top_left.x);
    let top = r1.top_left.y.min(r2.top_left.y);

    let right = (r1.top_left.x + r1.size.width as i32).max(r2.top_left.x + r2.size.width as i32);
    let bottom = (r1.top_left.y + r1.size.height as i32).max(r2.top_left.y + r2.size.height as i32);

    Rectangle::with_corners(Point::new(left, top), Point::new(right - 1, bottom - 1))
}





