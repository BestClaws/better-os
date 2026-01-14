use crate::color::Rgba8888;

pub trait Rasterizer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);

    // New high-level, pixel-format-agnostic APIs
    // Default implementations provided for basic functionality
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        // Default: bounds check only
        if x < 0 || y < 0 || x >= self.width() as i32 || y >= self.height() as i32 {
            return;
        }
        if coverage == 0 {
            return;
        }
        // Subclasses should override this with format-specific implementation
        unimplemented!("blend_pixel must be implemented by the rasterizer backend");
    }

    fn blend_hspan(&mut self, _x: i32, _y: i32, _colors: &[Rgba8888], _coverages: Option<&[u8]>) {
        unimplemented!("blend_hspan not implemented for this Rasterizer backend");
    }

    fn blend_hspan_with(
        &mut self,
        _x: i32,
        _y: i32,
        _len: i32,
        _f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        unimplemented!("blend_hspan_with not implemented for this Rasterizer backend");
    }

    fn blend_vspan_with(
        &mut self,
        _x: i32,
        _y: i32,
        _len: i32,
        _f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        unimplemented!("blend_vspan_with not implemented for this Rasterizer backend");
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        // Default implementation using blend_pixel with full coverage
        // Backends can override with optimized versions
        let x2 = x + w;
        let y2 = y + h;
        for py in y..y2 {
            for px in x..x2 {
                self.blend_pixel(px, py, color, 255);
            }
        }
    }

    fn stamp_rgb_zero_alpha(&mut self, _x: i32, _y: i32, _color: Rgba8888) {}

    /// Unsafe escape hatch to raw buffer. Primitives should not use this.
    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}

// Minimal scaffold for an RGB565 rasterizer backend.
// Not wired into the system yet; provided for future migration.
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

pub struct Rgb565Rasterizer {
    buffer: Vec<u8>,
    width: usize,
    height: usize,
}

impl Rgb565Rasterizer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: vec![0u8; width * height * 2],
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        let black: u16 = 0x0000;
        for px in self.buffer.chunks_exact_mut(2) {
            px[0] = (black >> 8) as u8;
            px[1] = black as u8;
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
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
    fn mark_dirty(&mut self, _min_x: i32, _min_y: i32, _max_x: i32, _max_y: i32) {}

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        if coverage == 0 {
            return;
        }
        if x < 0 || y < 0 {
            return;
        }
        let w = self.width as i32;
        let h = self.height as i32;
        if x >= w || y >= h {
            return;
        }

        // Convert Rgba8888 to RGB565 with alpha
        let rgba = color.to_u32();
        let r = ((rgba >> 24) & 0xFF) as u8;
        let g = ((rgba >> 16) & 0xFF) as u8;
        let b = ((rgba >> 8) & 0xFF) as u8;
        let a = (rgba & 0xFF) as u8;

        let r5 = (r as u16 >> 3) & 0x1F;
        let g6 = (g as u16 >> 2) & 0x3F;
        let b5 = (b as u16 >> 3) & 0x1F;
        let fg_rgb565 = (r5 << 11) | (g6 << 5) | b5;

        let eff = ((coverage as u32 * a as u32) / 255) as u8;
        if eff == 0 {
            return;
        }

        // RGB565 alpha blend
        let idx = ((y as usize) * self.width + (x as usize)) * 2;
        let bg = ((self.buffer[idx] as u16) << 8) | self.buffer[idx + 1] as u16;

        let out = if eff == 255 {
            fg_rgb565
        } else {
            let inv = 255 - eff;
            let br = ((bg >> 11) & 0x1F) * inv as u16;
            let bg_g = ((bg >> 5) & 0x3F) * inv as u16;
            let bb = (bg & 0x1F) * inv as u16;

            let fr = ((fg_rgb565 >> 11) & 0x1F) * eff as u16;
            let fg_g = ((fg_rgb565 >> 5) & 0x3F) * eff as u16;
            let fb = (fg_rgb565 & 0x1F) * eff as u16;

            (((br + fr) / 255) << 11) | (((bg_g + fg_g) / 255) << 5) | ((bb + fb) / 255)
        };

        self.buffer[idx] = (out >> 8) as u8;
        self.buffer[idx + 1] = out as u8;
    }

    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>) {
        if colors.is_empty() {
            return;
        }
        let len = colors.len() as i32;
        let mut f = |i: usize| {
            let cov = coverages.and_then(|c| c.get(i)).copied().unwrap_or(255);
            (colors[i], cov)
        };
        self.blend_hspan_with(x, y, len, &mut f);
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
        let w = self.width as i32;
        let h = self.height as i32;
        if y < 0 || y >= h {
            return;
        }
        let start = x.max(0);
        let end = (x + len - 1).min(w - 1);
        let skip = (start - x) as usize;
        let run_len = (end - start + 1) as usize;
        for i in 0..run_len {
            let (color, cov) = f(i + skip);
            self.blend_pixel(start + i as i32, y, color, cov);
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
        let w = self.width as i32;
        let h = self.height as i32;
        if x < 0 || x >= w {
            return;
        }
        let start = y.max(0);
        let end = (y + len - 1).min(h - 1);
        let skip = (start - y) as usize;
        let run_len = (end - start + 1) as usize;
        for i in 0..run_len {
            let (color, cov) = f(i + skip);
            self.blend_pixel(x, start + i as i32, color, cov);
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        if w <= 0 || h <= 0 {
            return;
        }
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w - 1).min(self.width as i32 - 1);
        let y1 = (y + h - 1).min(self.height as i32 - 1);
        for py in y0..=y1 {
            self.blend_hspan_with(x0, py, x1 - x0 + 1, |_i| (color, 255));
        }
    }
}
