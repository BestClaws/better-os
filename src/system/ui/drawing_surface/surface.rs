use alloc::vec::Vec;

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{blend_rgb565, rgba8888_to_rgb565_and_alpha};
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
                // Store RGB565 in big-endian (hi,lo) order using unaligned write.
                let (raw, _) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                unsafe {
                    // Convert to BE numeric so native write yields hi,lo bytes.
                    let be = raw.to_be();
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(idx) as *mut u16, be);
                }
            }
            fn get(buf: &[u8], idx: usize) -> Rgba8888 {
                // Read RGB565 stored in hi,lo order via unaligned u16 and fix endianness.
                let raw = unsafe {
                    let be = core::ptr::read_unaligned(buf.as_ptr().add(idx) as *const u16);
                    u16::from_be(be)
                };
                let r = (((raw >> 11) & 0x1F) as u16 * 527 + 23) >> 6;
                let g = (((raw >> 5) & 0x3F) as u16 * 259 + 33) >> 6;
                let b = ((raw & 0x1F) as u16 * 527 + 23) >> 6;
                Rgba8888::rgba(r as u8, g as u8, b as u8, 255)
            }
            fn blend(buf: &mut [u8], idx: usize, c: Rgba8888, coverage: u8) {
                // Unaligned u16 read/write with explicit BE order handling.
                let bg_raw: u16 = unsafe {
                    let be = core::ptr::read_unaligned(buf.as_ptr().add(idx) as *const u16);
                    u16::from_be(be)
                };
                let (fg_rgb565, a_src) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                let eff = ((coverage as u32 * a_src as u32) / 255) as u8;
                if eff == 0 { return; }
                let out = blend_rgb565(bg_raw, fg_rgb565, eff);
                unsafe {
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(idx) as *mut u16, out.to_be());
                }
            }
            fn encode_row(dst: &mut [u8], c: Rgba8888) {
                // Fill `dst` with the RGB565 representation of `c` using unsafe doubling.
                let (raw, _) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                let be = raw.to_be();
                unsafe {
                    let len = dst.len();
                    if len == 0 { return; }
                    // Write the first pixel (2 bytes)
                    core::ptr::write_unaligned(dst.as_mut_ptr() as *mut u16, be);
                    let mut filled = 2; // bytes filled
                    // Exponentially copy the written block to fill the buffer quickly.
                    while filled < len {
                        let copy_len = core::cmp::min(filled, len - filled);
                        core::ptr::copy_nonoverlapping(dst.as_ptr(), dst.as_mut_ptr().add(filled), copy_len);
                        filled += copy_len;
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
            fn noop_get(_: &[u8], _: usize) -> Rgba8888 { Rgba8888::rgba(0, 0, 0, 255) }
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

    /// Accessor for hot-path row encoder to avoid exposing private `ops`.
    #[inline(always)]
    pub(crate) fn encode_row_fn(&self) -> fn(&mut [u8], Rgba8888) {
        self.ops.encode_row
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
    }

    #[inline(always)]
    pub(crate) fn get_pixel_internal(&self, x: i32, y: i32) -> Rgba8888 {
        if x < 0 || y < 0 { return Rgba8888::rgba(0, 0, 0, 255); }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return Rgba8888::rgba(0, 0, 0, 255); }
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
    }
}
