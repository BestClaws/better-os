use core::marker::PhantomData;
use heapless::Vec;
use crate::libs::gfx::two_d::{Rasterizer, Rgb565, Rect, Point, Size};

/// A statically-safe, framebuffer-backed drawing canvas.
///
/// Supports dirty region tracking and pixel-level rendering. Efficient
/// for embedded systems that rely on partial screen updates.
///
/// Supported color format: `Rgb565`.
pub struct Canvas<'a> {
    buf: Option<&'a mut [u8]>,
    // buffer: &'a mut [u8],
    width: u32,
    height: u32,
    dirty_regions: Vec<Rect, 8>,
}

// ===== Canvas Implementation =====
impl<'a> Canvas<'a> {

    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: None,
            width,
            height,
            dirty_regions: {
                let mut v: Vec<Rect, 8> = Vec::new();
                v.push(Rect::new(Point::zero(), Size::new(width, height))).ok();
                v
            },
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
        self.dirty_regions.clear();
    }

    /// Returns the current dirty regions slice (may be empty).
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    /// Expands the dirty region to include the given rectangle.
    fn update_dirty(&mut self, new: Rect) {
        // Try to merge with existing regions to limit fragmentation
        let mut merged = false;
        for i in 0..self.dirty_regions.len() {
            let existing = self.dirty_regions[i];
            if existing.intersects(&new) {
                self.dirty_regions[i] = union_rect(existing, new);
                merged = true;
                break;
            }
        }

        if !merged {
            if self.dirty_regions.push(new).is_err() {
                // Fallback: if capacity exceeded, mark full-screen dirty
                self.dirty_regions.clear();
                self.dirty_regions.push(Rect::new(Point::zero(), Size::new(self.width, self.height))).ok();
            }
        }
    }
    
    
    
}

impl<'a> Canvas<'a> {
    pub fn resize(&mut self, width: u32, height: u32) {
        let required = (width * height * 2) as usize;
        assert!(self._buf_mut().len() >= required, "Buffer too small for Rgb565");

        self.width = width;
        self.height = height;
    }

    pub(crate) fn set_resources(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width * self.height * 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Rgb565");
        self.buf = Some(buffer);
    }

    pub fn clear_rgb(&mut self, color: Rgb565) {
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        for chunk in self._buf_mut().chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = hi;
                chunk[1] = lo;
            }
        }
        self.update_dirty(Rect::new(Point::zero(), Size::new(self.width, self.height)));
    }
}




/// Clips the given rectangle to canvas bounds and returns coordinates.
fn clip_rect(area: &Rect, width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x0 = area.top_left.x.max(0) as u32;
    let y0 = area.top_left.y.max(0) as u32;
    let x1 = (area.top_left.x as u32 + area.size.width).min(width);
    let y1 = (area.top_left.y as u32 + area.size.height).min(height);
    (x0, y0, x1, y1)
}

/// Computes the union (bounding box) of two rectangles.
fn union_rect(r1: Rect, r2: Rect) -> Rect {
    let left = r1.top_left.x.min(r2.top_left.x);
    let top = r1.top_left.y.min(r2.top_left.y);

    let right = (r1.top_left.x + r1.size.width as i32).max(r2.top_left.x + r2.size.width as i32);
    let bottom = (r1.top_left.y + r1.size.height as i32).max(r2.top_left.y + r2.size.height as i32);

    Rect::with_corners(Point::new(left, top), Point::new(right - 1, bottom - 1))
}
// ===== Rasterizer implementation =====
impl Rasterizer for Canvas<'_> {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb565) {
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        let idx = ((x + y * self.width) * 2) as usize;
        let raw = color.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
        self.update_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
    }
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        let idx = ((x + y * self.width) * 2) as usize;
        let hi = self._buf_mut()[idx] as u16;
        let lo = self._buf_mut()[idx + 1] as u16;
        let bg = Rgb565((hi << 8) | lo);
        let out = color.blend_over(bg, alpha);
        let raw = out.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
        self.update_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
    }
}




