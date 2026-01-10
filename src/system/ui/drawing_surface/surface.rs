use alloc::vec::Vec;

use super::util::{clip_rect, intersects_or_touches, union_rect};
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{blend_rgb565, rgba8888_to_gray4_and_alpha, rgba8888_to_rgb565_and_alpha};
use crate::system::hal::display::PixelFormat;
use crate::util::math::primitives::{Point, Rect, Size};

/// Pixel operation function pointers cached per format for hot paths.
pub struct PixelOps {
    pub bpp: usize,
    pub set_pixel: fn(buf: &mut [u8], pixel_idx: usize, color: Rgba8888),
    pub blend_pixel: fn(buf: &mut [u8], pixel_idx: usize, color: Rgba8888, coverage: u8),
    pub get_pixel: fn(buf: &[u8], pixel_idx: usize) -> Rgba8888,
    pub encode_row: fn(dst: &mut [u8], color: Rgba8888),
}

#[inline(always)]
fn ops_for_format(fmt: PixelFormat) -> PixelOps {
    match fmt {
        PixelFormat::Rgb565 => {
            // hi,lo ordering
            fn set(buf: &mut [u8], pixel_idx: usize, c: Rgba8888) {
                // Store RGB565 in big-endian (hi,lo) order using unaligned write.
                let byte_idx = pixel_idx * 2;
                let (raw, _) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                unsafe {
                    // Convert to BE numeric so native write yields hi,lo bytes.
                    let be = raw.to_be();
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(byte_idx) as *mut u16, be);
                }
            }
            fn get(buf: &[u8], pixel_idx: usize) -> Rgba8888 {
                // Read RGB565 stored in hi,lo order via unaligned u16 and fix endianness.
                let byte_idx = pixel_idx * 2;
                let raw = unsafe {
                    let be = core::ptr::read_unaligned(buf.as_ptr().add(byte_idx) as *const u16);
                    u16::from_be(be)
                };
                let r = (((raw >> 11) & 0x1F) as u16 * 527 + 23) >> 6;
                let g = (((raw >> 5) & 0x3F) as u16 * 259 + 33) >> 6;
                let b = ((raw & 0x1F) as u16 * 527 + 23) >> 6;
                Rgba8888::rgba(r as u8, g as u8, b as u8, 255)
            }
            fn blend(buf: &mut [u8], pixel_idx: usize, c: Rgba8888, coverage: u8) {
                // Unaligned u16 read/write with explicit BE order handling.
                let byte_idx = pixel_idx * 2;
                let bg_raw: u16 = unsafe {
                    let be = core::ptr::read_unaligned(buf.as_ptr().add(byte_idx) as *const u16);
                    u16::from_be(be)
                };
                let (fg_rgb565, a_src) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                let eff = ((coverage as u32 * a_src as u32) / 255) as u8;
                if eff == 0 {
                    return;
                }
                let out = blend_rgb565(bg_raw, fg_rgb565, eff);
                unsafe {
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(byte_idx) as *mut u16, out.to_be());
                }
            }
            fn encode_row(dst: &mut [u8], c: Rgba8888) {
                // Fill `dst` with the RGB565 representation of `c` using unsafe doubling.
                let (raw, _) = rgba8888_to_rgb565_and_alpha(c.to_u32());
                let be = raw.to_be();
                unsafe {
                    let len = dst.len();
                    if len == 0 {
                        return;
                    }
                    // Write the first pixel (2 bytes)
                    core::ptr::write_unaligned(dst.as_mut_ptr() as *mut u16, be);
                    let mut filled = 2; // bytes filled
                                        // Exponentially copy the written block to fill the buffer quickly.
                    while filled < len {
                        let copy_len = core::cmp::min(filled, len - filled);
                        core::ptr::copy_nonoverlapping(
                            dst.as_ptr(),
                            dst.as_mut_ptr().add(filled),
                            copy_len,
                        );
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
        PixelFormat::Gray4 => {
            // 4-bit grayscale: 2 pixels per byte, packed as [high nibble, low nibble]
            fn set(buf: &mut [u8], idx: usize, c: Rgba8888) {
                let (gray4, _) = rgba8888_to_gray4_and_alpha(c.to_u32());
                let byte_idx = idx / 2;
                let is_high = (idx & 1) == 0;
                if is_high {
                    buf[byte_idx] = (buf[byte_idx] & 0x0F) | (gray4 << 4);
                } else {
                    buf[byte_idx] = (buf[byte_idx] & 0xF0) | gray4;
                }
            }
            fn get(buf: &[u8], idx: usize) -> Rgba8888 {
                let byte_idx = idx / 2;
                let is_high = (idx & 1) == 0;
                let gray4 = if is_high {
                    (buf[byte_idx] >> 4) & 0x0F
                } else {
                    buf[byte_idx] & 0x0F
                };
                // Expand 4-bit to 8-bit: replicate the 4 bits
                let gray8 = (gray4 << 4) | gray4;
                Rgba8888::rgba(gray8, gray8, gray8, 255)
            }
            fn blend(buf: &mut [u8], idx: usize, c: Rgba8888, coverage: u8) {
                let byte_idx = idx / 2;
                let is_high = (idx & 1) == 0;
                let bg_gray4 = if is_high {
                    (buf[byte_idx] >> 4) & 0x0F
                } else {
                    buf[byte_idx] & 0x0F
                };
                let (fg_gray4, a_src) = rgba8888_to_gray4_and_alpha(c.to_u32());
                let eff = ((coverage as u32 * a_src as u32) / 255) as u8;
                if eff == 0 {
                    return;
                }
                // Expand to 8-bit for blending
                let bg8 = (bg_gray4 << 4) | bg_gray4;
                let fg8 = (fg_gray4 << 4) | fg_gray4;
                let blended8 = ((bg8 as u32 * (255 - eff) as u32 + fg8 as u32 * eff as u32) / 255) as u8;
                let blended4 = blended8 >> 4;
                if is_high {
                    buf[byte_idx] = (buf[byte_idx] & 0x0F) | (blended4 << 4);
                } else {
                    buf[byte_idx] = (buf[byte_idx] & 0xF0) | blended4;
                }
            }
            fn encode_row(dst: &mut [u8], c: Rgba8888) {
                let (gray4, _) = rgba8888_to_gray4_and_alpha(c.to_u32());
                let packed = (gray4 << 4) | gray4; // Both nibbles same value
                dst.fill(packed);
            }
            PixelOps {
                bpp: 1, // Note: this is per-pixel logical bpp, actual is 0.5 bytes
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
                Rgba8888::rgba(0, 0, 0, 255)
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
        let required = self.pixel_format.framebuffer_size(self.width, self.height);
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
    
    /// Get pixel at linear index (for blitter)
    pub(crate) fn get_pixel_at_index(&self, idx: usize) -> Rgba8888 {
        (self.ops.get_pixel)(self.buf(), idx)
    }
    
    /// Set pixel at linear index (for blitter)
    pub(crate) fn set_pixel_at_index(&mut self, idx: usize, color: Rgba8888) {
        (self.ops.set_pixel)(self.buf_mut(), idx, color);
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
    fn pixel_index(&self, x: u32, y: u32) -> usize {
        (x + y * self.width) as usize
    }

    // Format-specific fast paths for RGB565 (most common case)
    #[inline(always)]
    fn set_pixel_rgb565(&mut self, x: u32, y: u32, color: Rgba8888) {
        let byte_idx = ((x + y * self.width) as usize) << 1; // * 2 via shift
        let (raw, _) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        let buf = self.buf_mut();
        unsafe {
            let be = raw.to_be();
            core::ptr::write_unaligned(buf.as_mut_ptr().add(byte_idx) as *mut u16, be);
        }
    }

    #[inline(always)]
    fn blend_pixel_rgb565(&mut self, x: u32, y: u32, color: Rgba8888, coverage: u8) {
        let byte_idx = ((x + y * self.width) as usize) << 1;
        let buf = self.buf_mut();
        let bg_raw: u16 = unsafe {
            let be = core::ptr::read_unaligned(buf.as_ptr().add(byte_idx) as *const u16);
            u16::from_be(be)
        };
        let (fg_rgb565, a_src) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        let eff = ((coverage as u32 * a_src as u32) / 255) as u8;
        if eff == 0 {
            return;
        }
        let out = blend_rgb565(bg_raw, fg_rgb565, eff);
        unsafe {
            core::ptr::write_unaligned(buf.as_mut_ptr().add(byte_idx) as *mut u16, out.to_be());
        }
    }

    // Format-specific fast paths for Gray4
    #[inline(always)]
    fn set_pixel_gray4(&mut self, x: u32, y: u32, color: Rgba8888) {
        let pixel_idx = (x + y * self.width) as usize;
        let (gray4, _) = rgba8888_to_gray4_and_alpha(color.to_u32());
        let byte_idx = pixel_idx >> 1; // / 2 via shift
        let is_high = (pixel_idx & 1) == 0;
        let buf = self.buf_mut();
        if is_high {
            buf[byte_idx] = (buf[byte_idx] & 0x0F) | (gray4 << 4);
        } else {
            buf[byte_idx] = (buf[byte_idx] & 0xF0) | gray4;
        }
    }

    #[inline(always)]
    fn blend_pixel_gray4(&mut self, x: u32, y: u32, color: Rgba8888, coverage: u8) {
        let pixel_idx = (x + y * self.width) as usize;
        let byte_idx = pixel_idx >> 1;
        let is_high = (pixel_idx & 1) == 0;
        let buf = self.buf_mut();
        let bg_gray4 = if is_high {
            (buf[byte_idx] >> 4) & 0x0F
        } else {
            buf[byte_idx] & 0x0F
        };
        let (fg_gray4, a_src) = rgba8888_to_gray4_and_alpha(color.to_u32());
        let eff = ((coverage as u32 * a_src as u32) / 255) as u8;
        if eff == 0 {
            return;
        }
        let bg8 = (bg_gray4 << 4) | bg_gray4;
        let fg8 = (fg_gray4 << 4) | fg_gray4;
        let blended8 = ((bg8 as u32 * (255 - eff) as u32 + fg8 as u32 * eff as u32) / 255) as u8;
        let blended4 = blended8 >> 4;
        if is_high {
            buf[byte_idx] = (buf[byte_idx] & 0x0F) | (blended4 << 4);
        } else {
            buf[byte_idx] = (buf[byte_idx] & 0xF0) | blended4;
        }
    }

    // Internal hot paths used by Rasterizer impl - dispatch to format-specific fast paths
    #[inline(always)]
    pub(crate) fn set_pixel_internal(&mut self, x: i32, y: i32, color: Rgba8888) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return;
        }
        // Fast path dispatch based on format
        match self.pixel_format {
            PixelFormat::Rgb565 => self.set_pixel_rgb565(x, y, color),
            PixelFormat::Gray4 => self.set_pixel_gray4(x, y, color),
            _ => {
                let pixel_idx = self.pixel_index(x, y);
                (self.ops.set_pixel)(self.buf_mut(), pixel_idx, color);
            }
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
        // Fast path dispatch based on format
        match self.pixel_format {
            PixelFormat::Rgb565 => self.blend_pixel_rgb565(x, y, color, coverage),
            PixelFormat::Gray4 => self.blend_pixel_gray4(x, y, color, coverage),
            _ => {
                let pixel_idx = self.pixel_index(x, y);
                (self.ops.blend_pixel)(self.buf_mut(), pixel_idx, color, coverage);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn get_pixel_internal(&self, x: i32, y: i32) -> Rgba8888 {
        if x < 0 || y < 0 {
            return Rgba8888::rgba(0, 0, 0, 255);
        }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height {
            return Rgba8888::rgba(0, 0, 0, 255);
        }
        let pixel_idx = self.pixel_index(x, y);
        (self.ops.get_pixel)(self.buf(), pixel_idx)
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
        let y_offset = y as u32 * self.width;
        let setp = self.ops.set_pixel;
        let buf = self.buf_mut();
        for i in 0..actual_width as usize {
            let pixel_idx = (start_x as usize + i + y_offset as usize);
            setp(buf, pixel_idx, color);
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
        let setp = self.ops.set_pixel;
        let width = self.width; // avoid borrow of self during loop
        let buf = self.buf_mut();
        for i in 0..actual_height as usize {
            let pixel_idx = x as usize + (start_y as usize + i) * width as usize;
            setp(buf, pixel_idx, color);
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
        let width = self.width;
        let setp = self.ops.set_pixel;
        let buf = self.buf_mut();
        for y in clipped_rect.top_left.y..=clipped_rect.bottom() {
            let y_offset = y as usize * width as usize;
            for x in clipped_rect.top_left.x..=clipped_rect.right() {
                let pixel_idx = x as usize + y_offset;
                setp(buf, pixel_idx, color);
            }
        }
    }
}
