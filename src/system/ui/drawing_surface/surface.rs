use alloc::vec::Vec;

use crate::libs::gfx::two_d::{Rasterizer, Rgba8888};
use crate::system::hal::display::PixelFormat;
use crate::util::math::primitives::{Point, Rect, Size};
use super::util::{clip_rect, intersects_or_touches, union_rect};

/// Pixel operation function pointers cached per format for hot paths.
pub struct PixelOps {
    pub bpp: usize,
    pub set_pixel: fn(buf: &mut [u8], byte_idx: usize, color: Rgba8888),
    pub blend_pixel: fn(buf: &mut [u8], byte_idx: usize, color: Rgba8888, coverage: u8),
    pub get_pixel: fn(buf: &[u8], byte_idx: usize) -> Rgba8888,
    pub encode_row: fn(dst: &mut [u8], color: Rgba8888),
}

#[inline(always)]
fn ops_for_format(fmt: PixelFormat) -> PixelOps {
    match fmt {
        PixelFormat::Rgb565 => {
            // hi,lo ordering
            fn set(buf: &mut [u8], idx: usize, c: Rgba8888) {
                let r5: u16 = ((c.r as u16) >> 3) & 0x1F;
                let g6: u16 = ((c.g as u16) >> 2) & 0x3F;
                let b5: u16 = ((c.b as u16) >> 3) & 0x1F;
                let raw: u16 = (r5 << 11) | (g6 << 5) | b5;
                buf[idx] = (raw >> 8) as u8;
                buf[idx + 1] = raw as u8;
            }
            fn get(buf: &[u8], idx: usize) -> Rgba8888 {
                let hi = buf[idx] as u16;
                let lo = buf[idx + 1] as u16;
                let raw = (hi << 8) | lo;
                let r = (((raw >> 11) & 0x1F) as u16 * 527 + 23) >> 6;
                let g = (((raw >> 5) & 0x3F) as u16 * 259 + 33) >> 6;
                let b = ((raw & 0x1F) as u16 * 527 + 23) >> 6;
                Rgba8888::opaque(r as u8, g as u8, b as u8)
            }
            fn blend(buf: &mut [u8], idx: usize, c: Rgba8888, coverage: u8) {
                // Read bg
                let hi = buf[idx] as u16;
                let lo = buf[idx + 1] as u16;
                let bg_raw = (hi << 8) | lo;
                let br = (((bg_raw >> 11) & 0x1F) as u16 * 527 + 23) >> 6;
                let gg = (((bg_raw >> 5) & 0x3F) as u16 * 259 + 33) >> 6;
                let bb = ((bg_raw & 0x1F) as u16 * 527 + 23) >> 6;
                // Compose alpha
                let cov = coverage as u32;
                let a_src = c.a as u32;
                let a = ((cov * a_src + 127) / 255) as u8;
                let ia = 255 - a as u16;
                let r = ((c.r as u16 * a as u16 + br as u16 * ia + 127) / 255) as u8;
                let g = ((c.g as u16 * a as u16 + gg as u16 * ia + 127) / 255) as u8;
                let b = ((c.b as u16 * a as u16 + bb as u16 * ia + 127) / 255) as u8;
                let raw: u16 =
                    (((r as u16) >> 3) << 11) | (((g as u16) >> 2) << 5) | ((b as u16) >> 3);
                buf[idx] = (raw >> 8) as u8;
                buf[idx + 1] = raw as u8;
            }
            fn encode_row(dst: &mut [u8], c: Rgba8888) {
                let r5: u16 = ((c.r as u16) >> 3) & 0x1F;
                let g6: u16 = ((c.g as u16) >> 2) & 0x3F;
                let b5: u16 = ((c.b as u16) >> 3) & 0x1F;
                let raw: u16 = (r5 << 11) | (g6 << 5) | b5;
                let hi = (raw >> 8) as u8;
                let lo = raw as u8;
                for chunk in dst.chunks_mut(2) {
                    if chunk.len() == 2 {
                        chunk[0] = hi;
                        chunk[1] = lo;
                    }
                }
            }
            PixelOps {
                bpp: 2,
                set_pixel: set,
                blend_pixel: blend,
                get_pixel: get,
                encode_row,
            }
        }
        _ => {
            debug_assert!(false, "Unsupported PixelFormat not implemented in PixelOps");
            fn noop_set(_: &mut [u8], _: usize, _: Rgba8888) {}
            fn noop_blend(_: &mut [u8], _: usize, _: Rgba8888, _: u8) {}
            fn noop_get(_: &[u8], _: usize) -> Rgba8888 {
                Rgba8888::opaque(0, 0, 0)
            }
            fn noop_row(_: &mut [u8], _: Rgba8888) {}
            PixelOps {
                bpp: 1,
                set_pixel: noop_set,
                blend_pixel: noop_blend,
                get_pixel: noop_get,
                encode_row: noop_row,
            }
        }
    }
}

/// Framebuffer-backed drawing surface with dirty region tracking.
pub struct DrawingSurface<'a> {
    buf: Option<&'a mut [u8]>,
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
    ops: PixelOps,
    dirty_regions: Vec<Rect>,
    max_dirty_regions: usize,
    batched_dirty_bounds: Option<Rect>,
}

impl<'a> DrawingSurface<'a> {
    /// Create a new unattached surface. Attach buffer later via `attach_buffer`.
    pub fn new_unattached(width: u32, height: u32, format: PixelFormat) -> Self {
        let ops = ops_for_format(format);
        Self {
            buf: None,
            width,
            height,
            pixel_format: format,
            ops,
            dirty_regions: {
                let mut v: Vec<Rect> = Vec::new();
                v.push(Rect::new(Point::zero(), Size::new(width, height)));
                v
            },
            max_dirty_regions: 8,
            batched_dirty_bounds: None,
        }
    }

    /// Create a surface and attach the provided buffer immediately.
    pub fn new_with_buffer(
        width: u32,
        height: u32,
        format: PixelFormat,
        buffer: &'a mut [u8],
    ) -> Self {
        let mut s = Self::new_unattached(width, height, format);
        s.attach_buffer(buffer);
        s
    }

    /// Attach a framebuffer. Must satisfy capacity: `width * height * bpp`.
    pub fn attach_buffer(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width as usize) * (self.height as usize) * self.ops.bpp;
        assert!(
            buffer.len() >= required,
            "Buffer too small for DrawingSurface"
        );
        self.buf = Some(buffer);
    }

    /// Detach the framebuffer.
    pub fn detach_buffer(&mut self) {
        self.buf = None;
    }

    pub fn reconfigure(&mut self, width: u32, height: u32, format: PixelFormat) {
        self.width = width;
        self.height = height;
        self.pixel_format = format;
        self.ops = ops_for_format(format);
    }

    /// Set maximum number of dirty regions to retain before falling back
    /// to full-surface dirty. Default: 8.
    pub fn set_max_dirty_regions(&mut self, max: usize) {
        self.max_dirty_regions = max.max(1);
    }

    #[inline(always)]
    fn buf(&self) -> &[u8] {
        self.buf
            .as_ref()
            .map(|r| &**r)
            .expect("Buffer not initialized")
    }
    #[inline(always)]
    fn buf_mut(&mut self) -> &mut [u8] {
        self.buf
            .as_mut()
            .map(|r| &mut **r)
            .expect("Buffer not initialized")
    }

    /// Immutable access to raw buffer.
    pub fn buffer(&self) -> &[u8] {
        self.buf()
    }
    /// Mutable access to raw buffer.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.buf_mut()
    }
    /// Surface width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }
    /// Surface height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
    /// Bytes per pixel of the attached format.
    pub fn bytes_per_pixel(&self) -> usize {
        self.ops.bpp
    }
    /// Current pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn flush(&mut self) {
        self.dirty_regions.clear();
    }
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    /// Start a batched operation to accumulate dirty bounds.
    pub fn begin_drawing_batch(&mut self, bounds: Rect) {
        self.batched_dirty_bounds = Some(bounds);
    }
    pub fn end_drawing_batch(&mut self) {
        if let Some(bounds) = self.batched_dirty_bounds.take() {
            self.mark_region_dirty(bounds);
        }
    }

    pub fn mark_dirty(&mut self, area: Rect) {
        self.mark_region_dirty(area);
    }

    fn mark_region_dirty(&mut self, mut new_region: Rect) {
        let (x0, y0, x1, y1) = clip_rect(&new_region, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        new_region = Rect::new(
            Point::new(x0 as i32, y0 as i32),
            Size::new(x1 - x0, y1 - y0),
        );
        if let Some(ref mut bounds) = self.batched_dirty_bounds {
            *bounds = union_rect(*bounds, new_region);
            return;
        }
        let mut i = 0;
        while i < self.dirty_regions.len() {
            let current = self.dirty_regions[i];
            if intersects_or_touches(&current, &new_region) {
                new_region = union_rect(current, new_region);
                self.dirty_regions.swap_remove(i);
                i = 0;
                continue;
            }
            i += 1;
        }
        self.dirty_regions.push(new_region);
        if self.dirty_regions.len() > self.max_dirty_regions {
            self.dirty_regions.clear();
            self.dirty_regions
                .push(Rect::new(Point::zero(), Size::new(self.width, self.height)));
        }
    }

    /// Clear entire surface with a solid color.
    pub fn clear(&mut self, color: Rgba8888) {
        let encode = self.ops.encode_row;
        let buf = self.buf_mut();
        encode(buf, color);
        self.mark_region_dirty(Rect::new(Point::zero(), Size::new(self.width, self.height)));
    }

    #[inline(always)]
    fn pixel_byte_index(&self, x: u32, y: u32) -> usize {
        ((x + y * self.width) as usize) * self.ops.bpp
    }

    // Internal hot paths used by Rasterizer impl
    #[inline(always)]
    pub(crate) fn set_pixel_internal(&mut self, x: i32, y: i32, color: Rgba8888) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.pixel_byte_index(x, y);
        (self.ops.set_pixel)(self.buf_mut(), idx, color);
        if self.batched_dirty_bounds.is_none() {
            self.mark_region_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
        }
    }

    #[inline(always)]
    pub(crate) fn blend_pixel_internal(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.pixel_byte_index(x, y);
        (self.ops.blend_pixel)(self.buf_mut(), idx, color, coverage);
        if self.batched_dirty_bounds.is_none() {
            self.mark_region_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
        }
    }

    #[inline(always)]
    pub(crate) fn get_pixel_internal(&self, x: i32, y: i32) -> Rgba8888 {
        if x < 0 || y < 0 {
            return Rgba8888::opaque(0, 0, 0);
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return Rgba8888::opaque(0, 0, 0);
        }
        let idx = self.pixel_byte_index(x, y);
        (self.ops.get_pixel)(self.buf(), idx)
    }

    pub(crate) fn set_pixels_horizontal_internal(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        color: Rgba8888,
    ) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        if x < 0 || x + width as i32 > self.width as i32 {
            return;
        }
        let start_x = x.max(0) as u32;
        let end_x = (x + width as i32).min(self.width as i32) as u32;
        let actual_width = end_x - start_x;
        if actual_width == 0 {
            return;
        }
        let bpp = self.ops.bpp;
        let y_offset = y as u32 * self.width;
        let start_idx = ((start_x + y_offset) as usize) * bpp;
        let mut tmp = [0u8; 2];
        let setp = self.ops.set_pixel;
        setp(&mut tmp, 0, color);
        let buf = self.buf_mut();
        for i in 0..actual_width as usize {
            let idx = start_idx + i * bpp;
            buf[idx] = tmp[0];
            if bpp > 1 {
                buf[idx + 1] = tmp[1];
            }
        }
        if self.batched_dirty_bounds.is_none() {
            self.mark_region_dirty(Rect::new(
                Point::new(start_x as i32, y),
                Size::new(actual_width, 1),
            ));
        }
    }

    pub(crate) fn set_pixels_vertical_internal(
        &mut self,
        x: i32,
        y: i32,
        height: u32,
        color: Rgba8888,
    ) {
        if x < 0 || x >= self.width as i32 {
            return;
        }
        if y < 0 || y + height as i32 > self.height as i32 {
            return;
        }
        let start_y = y.max(0) as u32;
        let end_y = (y + height as i32).min(self.height as i32) as u32;
        let actual_height = end_y - start_y;
        if actual_height == 0 {
            return;
        }
        let bpp = self.ops.bpp;
        let mut tmp = [0u8; 2];
        let setp = self.ops.set_pixel;
        setp(&mut tmp, 0, color);
        let width = self.width; // avoid borrow of self during loop
        let buf = self.buf_mut();
        for i in 0..actual_height as usize {
            let idx = ((x as u32 + (start_y + i as u32) * width) as usize) * bpp;
            buf[idx] = tmp[0];
            if bpp > 1 {
                buf[idx + 1] = tmp[1];
            }
        }
        if self.batched_dirty_bounds.is_none() {
            self.mark_region_dirty(Rect::new(
                Point::new(x, start_y as i32),
                Size::new(1, actual_height),
            ));
        }
    }

    pub(crate) fn set_pixels_rect_internal(&mut self, rect: Rect, color: Rgba8888) {
        if rect.size.width == 0 || rect.size.height == 0 {
            return;
        }
        let clip = Rect::new(Point::zero(), Size::new(self.width, self.height));
        let Some(clipped_rect) = rect.intersection(clip) else {
            return;
        };
        let bpp = self.ops.bpp;
        let width = self.width;
        let mut tmp = [0u8; 2];
        let setp = self.ops.set_pixel;
        setp(&mut tmp, 0, color);
        let buf = self.buf_mut();
        for y in clipped_rect.top_left.y..=clipped_rect.bottom() {
            let y_offset = y as u32 * width;
            for x in clipped_rect.top_left.x..=clipped_rect.right() {
                let idx = ((x as u32 + y_offset) as usize) * bpp;
                buf[idx] = tmp[0];
                if bpp > 1 {
                    buf[idx + 1] = tmp[1];
                }
            }
        }
        if self.batched_dirty_bounds.is_none() {
            self.mark_region_dirty(clipped_rect);
        }
    }
}
