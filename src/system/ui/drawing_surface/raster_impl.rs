//! Rasterizer trait implementation for `DrawingSurface` using the new gfx API.
//!
//! All color inputs are `Rgba8888`; storage is in the negotiated native
//! pixel format (reference: RGB565). Dirty regions are reported explicitly
//! via `mark_dirty` and not computed per-pixel.
use super::surface::DrawingSurface;
use rust_gfx::color::Rgba8888;
use rust_gfx::rasterizer::Rasterizer;
use crate::util::math::primitives::Rect;

impl Rasterizer for DrawingSurface<'_> {
    fn width(&self) -> usize {
        self.width() as usize
    }
    fn height(&self) -> usize {
        self.height() as usize
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }

    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) {
        let w = self.width() as i32;
        let h = self.height() as i32;
        if min_x >= max_x || min_y >= max_y {
            return;
        }
        let x0 = min_x.max(0).min(w);
        let y0 = min_y.max(0).min(h);
        let x1 = max_x.max(0).min(w);
        let y1 = max_y.max(0).min(h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let rect = Rect::with_corners(
            crate::util::math::primitives::Point::new(x0, y0),
            crate::util::math::primitives::Point::new(x1 - 1, y1 - 1),
        );
        self.mark_dirty(rect);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        self.blend_pixel_internal(x, y, color, coverage);
    }

    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>) {
        if colors.is_empty() {
            return;
        }
        let len = colors
            .len()
            .min(coverages.map(|c| c.len()).unwrap_or(colors.len())) as i32;
        self.blend_hspan_with(x, y, len, |i| {
            let c = colors[i];
            let cov = coverages.and_then(|cvs| cvs.get(i)).copied().unwrap_or(255);
            (c, cov)
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
        let w = self.width() as i32;
        let h = self.height() as i32;
        if y < 0 || y >= h {
            return;
        }
        let start = x.max(0);
        let end = (x + len - 1).min(w - 1);
        let skip = (start - x) as usize;
        let run_len = (end - start + 1) as usize;
        for i in 0..run_len {
            let (color, cov) = f(i + skip);
            self.blend_pixel_internal(start + i as i32, y, color, cov);
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
        let w = self.width() as i32;
        let h = self.height() as i32;
        if x < 0 || x >= w {
            return;
        }
        let start = y.max(0);
        let end = (y + len - 1).min(h - 1);
        let skip = (start - y) as usize;
        let run_len = (end - start + 1) as usize;
        for i in 0..run_len {
            let (color, cov) = f(i + skip);
            self.blend_pixel_internal(x, start + i as i32, color, cov);
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        if w <= 0 || h <= 0 {
            return;
        }
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w - 1).min(self.width() as i32 - 1);
        let y1 = (y + h - 1).min(self.height() as i32 - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }
        let a = (color.to_u32() & 0xFF) as u8;

        // Fast path for fully opaque fill
        if a == 255 {
            // Use set_pixels_horizontal_internal for each row to properly handle all formats
            for py in y0..=y1 {
                self.set_pixels_horizontal_internal(x0, py, (x1 - x0 + 1) as u32, color);
            }
        } else {
            // Fallback to per-pixel blend for alpha < 255.
            for py in y0..=y1 {
                self.blend_hspan_with(x0, py, x1 - x0 + 1, |i| (color, 255));
            }
        }
    }

    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}
