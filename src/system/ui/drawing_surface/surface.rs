use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::NonNull;

use super::util::{clip_rect, intersects_or_touches, union_rect};
use crate::system::hal::display::PixelFormat;
use crate::util::math::primitives::Rect;
use rust_gfx::color::Rgba8888;

const MAX_DIRTY_REGIONS: usize = 16;

#[inline]
pub(crate) fn rgba8888_to_rgb565(color: Rgba8888) -> u16 {
    let r = color.r() as u16;
    let g = color.g() as u16;
    let b = color.b() as u16;
    let r5 = ((r * 31) + 127) / 255;
    let g6 = ((g * 63) + 127) / 255;
    let b5 = ((b * 31) + 127) / 255;
    (r5 << 11) | (g6 << 5) | b5
}

#[inline]
pub(crate) fn rgb565_to_rgba8888(pixel: u16) -> Rgba8888 {
    let r5 = (pixel >> 11) & 0x1F;
    let g6 = (pixel >> 5) & 0x3F;
    let b5 = pixel & 0x1F;
    let r = ((r5 * 255) + 15) / 31;
    let g = ((g6 * 255) + 31) / 63;
    let b = ((b5 * 255) + 15) / 31;
    Rgba8888::rgba(r as u8, g as u8, b as u8, 255)
}

#[inline]
pub(crate) fn gray4_to_rgba8888(gray: u8) -> Rgba8888 {
    let value = (gray as u16 * 17) as u8;
    Rgba8888::rgba(value, value, value, 255)
}

#[inline]
pub(crate) fn rgba8888_to_gray4(color: Rgba8888) -> u8 {
    let r = color.r() as u32;
    let g = color.g() as u32;
    let b = color.b() as u32;
    let luma = (r * 54 + g * 183 + b * 18 + 127) / 255;
    ((luma * 15 + 127) / 255) as u8
}

/// Framebuffer-backed drawing surface with dirty region tracking.
pub struct DrawingSurface<'a> {
    buf: Option<NonNull<u8>>,
    buf_len: usize,
    _marker: PhantomData<&'a mut [u8]>,
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
    dirty_regions: Vec<Rect>,
}

impl<'a> DrawingSurface<'a> {
    /// Create a new unattached surface. Attach buffer later via `attach_buffer`.
    pub fn new_unattached(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            buf: None,
            buf_len: 0,
            _marker: PhantomData,
            width,
            height,
            pixel_format: format,
            dirty_regions: Vec::with_capacity(4),
        }
    }

    /// Create a surface and attach the provided buffer immediately.
    pub fn new_with_buffer(
        width: u32,
        height: u32,
        format: PixelFormat,
        buffer: &'a mut [u8],
    ) -> Self {
        let mut surface = Self::new_unattached(width, height, format);
        surface.attach_buffer(buffer);
        surface
    }

    /// Attach a framebuffer. Must satisfy capacity: `width * height * bpp`.
    pub fn attach_buffer(&mut self, buffer: &'a mut [u8]) {
        let required = self.pixel_format.framebuffer_size(self.width, self.height);
        assert!(
            buffer.len() >= required,
            "Buffer too small for DrawingSurface"
        );
        self.buf = NonNull::new(buffer.as_mut_ptr());
        self.buf_len = buffer.len();
    }

    /// Detach the framebuffer.
    pub fn detach_buffer(&mut self) {
        self.buf = None;
        self.buf_len = 0;
    }

    pub fn reconfigure(&mut self, width: u32, height: u32, format: PixelFormat) {
        self.width = width;
        self.height = height;
        self.pixel_format = format;
        self.dirty_regions.clear();

        if let Some(_) = self.buf {
            let required = self.pixel_format.framebuffer_size(self.width, self.height);
            assert!(
                self.buf_len >= required,
                "Attached buffer too small for reconfigured DrawingSurface"
            );
        }
    }

    /// Immutable access to raw buffer.
    pub fn buffer(&self) -> &[u8] {
        let ptr = self
            .buf
            .expect("DrawingSurface buffer not attached")
            .as_ptr();
        let required = self.pixel_format.framebuffer_size(self.width, self.height);
        unsafe { core::slice::from_raw_parts(ptr as *const u8, required) }
    }
    /// Mutable access to raw buffer.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        let ptr = self
            .buf
            .expect("DrawingSurface buffer not attached")
            .as_ptr();
        let required = self.pixel_format.framebuffer_size(self.width, self.height);
        unsafe { core::slice::from_raw_parts_mut(ptr, required) }
    }
    /// Check if a framebuffer is attached.
    pub fn is_attached(&self) -> bool {
        self.buf.is_some()
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
        self.pixel_format.bytes_per_pixel()
    }
    /// Current pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }
    pub fn clear(&mut self, color: Rgba8888) {
        if self.width == 0 || self.height == 0 {
            self.flush();
            return;
        }
        if self.buf.is_none() {
            self.flush();
            return;
        }

        match self.pixel_format {
            PixelFormat::Rgb565 => {
                let value = rgba8888_to_rgb565(color);
                let hi = (value >> 8) as u8;
                let lo = (value & 0xFF) as u8;
                let buf = self.buffer_mut();
                for chunk in buf.chunks_exact_mut(2) {
                    chunk[0] = hi;
                    chunk[1] = lo;
                }
            }
            PixelFormat::Gray4 => {
                let gray = rgba8888_to_gray4(color);
                let packed = (gray << 4) | gray;
                self.buffer_mut().fill(packed);
            }
        }

        self.dirty_regions.clear();
        let rect = Rect::from_coords(0, 0, self.width, self.height);
        self.mark_dirty_clipped(rect);
    }

    pub fn flush(&mut self) {
        self.dirty_regions.clear();
    }
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    pub fn get_pixel_at_index(&self, index: usize) -> Rgba8888 {
        if self.buf.is_none() {
            return Rgba8888::TRANSPARENT;
        }
        match self.pixel_format {
            PixelFormat::Rgb565 => {
                let byte_index = index * 2;
                let buf = self.buffer();
                if byte_index + 1 >= buf.len() {
                    return Rgba8888::TRANSPARENT;
                }
                let value = u16::from_be_bytes([buf[byte_index], buf[byte_index + 1]]);
                rgb565_to_rgba8888(value)
            }
            PixelFormat::Gray4 => {
                let buf = self.buffer();
                let byte_index = index / 2;
                if byte_index >= buf.len() {
                    return Rgba8888::TRANSPARENT;
                }
                let high = (index & 1) == 0;
                let byte = buf[byte_index];
                let gray = if high { byte >> 4 } else { byte & 0x0F };
                gray4_to_rgba8888(gray)
            }
        }
    }

    pub fn set_pixel_at_index(&mut self, index: usize, color: Rgba8888) {
        if self.buf.is_none() {
            return;
        }
        match self.pixel_format {
            PixelFormat::Rgb565 => {
                let buf = self.buffer_mut();
                let byte_index = index * 2;
                if byte_index + 1 >= buf.len() {
                    return;
                }
                let value = rgba8888_to_rgb565(color);
                buf[byte_index] = (value >> 8) as u8;
                buf[byte_index + 1] = (value & 0xFF) as u8;
            }
            PixelFormat::Gray4 => {
                let buf = self.buffer_mut();
                let byte_index = index / 2;
                if byte_index >= buf.len() {
                    return;
                }
                let gray = rgba8888_to_gray4(color) & 0x0F;
                let high = (index & 1) == 0;
                let byte = &mut buf[byte_index];
                if high {
                    *byte = (*byte & 0x0F) | (gray << 4);
                } else {
                    *byte = (*byte & 0xF0) | gray;
                }
            }
        }
    }

    pub(crate) fn pixel_index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        let (width, height) = (self.width as i32, self.height as i32);
        if x >= width || y >= height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn accumulate_dirty(&mut self, rect: Rect) {
        let mut pending = rect;
        let mut i = 0;
        while i < self.dirty_regions.len() {
            if intersects_or_touches(&self.dirty_regions[i], &pending) {
                let merged = union_rect(self.dirty_regions[i], pending);
                self.dirty_regions.remove(i);
                pending = merged;
            } else {
                i += 1;
            }
        }

        if self.dirty_regions.len() < MAX_DIRTY_REGIONS {
            self.dirty_regions.push(pending);
        } else {
            let mut collapsed = pending;
            for existing in self.dirty_regions.iter() {
                collapsed = union_rect(*existing, collapsed);
            }
            self.dirty_regions.clear();
            self.dirty_regions.push(collapsed);
        }
    }

    pub(crate) fn mark_dirty_clipped(&mut self, rect: Rect) {
        let (x0, y0, x1, y1) = clip_rect(&rect, self.width, self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let clipped = Rect::from_coords(x0 as i32, y0 as i32, x1 - x0, y1 - y0);
        self.accumulate_dirty(clipped);
    }
}
