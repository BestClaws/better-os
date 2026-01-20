extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::color::Rgba8888;
use crate::Rasterizer;

/// Simple RGB565 rasterizer implementation backed by an owned pixel buffer.
pub struct Rgb565Rasterizer {
    buffer: Vec<u8>,
    width: usize,
    height: usize,
}

impl Rgb565Rasterizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0; width * height * 2],
            width,
            height,
        }
    }

    #[inline]
    fn pixel_index(&self, x: i32, y: i32) -> usize {
        debug_assert!(x >= 0 && y >= 0, "pixel_index called with negative coordinate");
        ((y as usize) * self.width + x as usize) * 2
    }

    /// Immutable access to the underlying RGB565 buffer (hi,lo byte order per pixel).
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Mutable access to the underlying RGB565 buffer (hi,lo byte order per pixel).
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

}

impl Rasterizer for Rgb565Rasterizer {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    fn clear(&mut self, color: Rgba8888) {
        let (raw, _) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        for px in self.buffer.chunks_exact_mut(2) {
            px[0] = hi;
            px[1] = lo;
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        if coverage == 0 || !self.in_bounds(x, y) {
            return;
        }
        let (fg_raw, alpha) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        let eff = ((coverage as u32 * alpha as u32) / 255) as u8;
        if eff == 0 {
            return;
        }
        let idx = self.pixel_index(x, y);
        let bg = ((self.buffer[idx] as u16) << 8) | self.buffer[idx + 1] as u16;
        let out = blend_rgb565(bg, fg_raw, eff);
        self.buffer[idx] = (out >> 8) as u8;
        self.buffer[idx + 1] = out as u8;
    }

    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>) {
        if colors.is_empty() {
            return;
        }
        let len = colors.len().min(coverages.map(|c| c.len()).unwrap_or(colors.len())) as i32;
        self.blend_hspan_with(x, y, len, |i| {
            let coverage = coverages.and_then(|cov| cov.get(i)).copied().unwrap_or(255);
            (colors[i], coverage)
        });
    }

    fn blend_hspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        if len <= 0 {
            return;
        }
        if y < 0 || y >= self.height_i32() {
            return;
        }
        if let Some(span) = self.clamp_horizontal_span(x, len) {
            for i in 0..span.len() {
                let (color, coverage) = f(i + span.skip);
                self.blend_pixel(span.start + i as i32, y, color, coverage);
            }
        }
    }

    fn blend_vspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        if len <= 0 {
            return;
        }
        if x < 0 || x >= self.width_i32() {
            return;
        }
        if let Some(span) = self.clamp_vertical_span(y, len) {
            for i in 0..span.len() {
                let (color, coverage) = f(i + span.skip);
                self.blend_pixel(x, span.start + i as i32, color, coverage);
            }
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        if let Some(region) = self.clamp_rect_from_size(x, y, w, h) {
            let (raw, _) = rgba8888_to_rgb565_and_alpha(color.to_u32());
            let hi = (raw >> 8) as u8;
            let lo = raw as u8;
            for py in region.min_y..region.max_y {
                let mut idx = self.pixel_index(region.min_x, py);
                for _ in region.min_x..region.max_x {
                    self.buffer[idx] = hi;
                    self.buffer[idx + 1] = lo;
                    idx += 2;
                }
            }
        }
    }

    fn stamp_rgb_zero_alpha(&mut self, x: i32, y: i32, color: Rgba8888) {
        // RGB565 cannot encode alpha; treat as a regular blend with zero coverage (no-op)
        let _ = (x, y, color);
    }

    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}

#[inline(always)]
fn rgba8888_to_rgb565_and_alpha(rgba: u32) -> (u16, u8) {
    let r = ((rgba >> 24) & 0xFF) as u8;
    let g = ((rgba >> 16) & 0xFF) as u8;
    let b = ((rgba >> 8) & 0xFF) as u8;
    let a = (rgba & 0xFF) as u8;

    let r5 = (r as u16 >> 3) & 0x1F;
    let g6 = (g as u16 >> 2) & 0x3F;
    let b5 = (b as u16 >> 3) & 0x1F;

    let rgb565 = (r5 << 11) | (g6 << 5) | b5;
    (rgb565, a)
}

#[inline(always)]
fn blend_rgb565(bg: u16, fg: u16, opa: u8) -> u16 {
    if opa == 255 {
        return fg;
    }
    if opa == 0 {
        return bg;
    }

    let inv = 255 - opa;

    let br = ((bg >> 11) & 0x1F) * inv as u16;
    let bg_g = ((bg >> 5) & 0x3F) * inv as u16;
    let bb = (bg & 0x1F) * inv as u16;

    let fr = ((fg >> 11) & 0x1F) * opa as u16;
    let fg_g = ((fg >> 5) & 0x3F) * opa as u16;
    let fb = (fg & 0x1F) * opa as u16;

    (((br + fr) / 255) << 11) | (((bg_g + fg_g) / 255) << 5) | ((bb + fb) / 255)
}
