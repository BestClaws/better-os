use crate::colors::Color;

pub trait Rasterizer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];
    fn mark_dirty(&mut self, min_x: i32, min_y: i32, max_x: i32, max_y: i32);

    /// Clear entire surface with a solid color.
    fn clear(&mut self, color: Color);



    fn blend_pixel(&mut self, x: i32, y: i32, color: Color, coverage: u8) {
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

    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Color], coverages: Option<&[u8]>) {
        if colors.is_empty() {
            return;
        }
        let len = colors.len() as i32;
        self.blend_hspan_with(x, y, len, |i| {
            let coverage = coverages
                .and_then(|c| c.get(i))
                .copied()
                .unwrap_or(255);
            (colors[i], coverage)
        });
    }

    fn blend_hspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Color, u8),
    ) {
        if len <= 0 {
            return;
        }
        for i in 0..len as usize {
            let (color, coverage) = f(i);
            if coverage == 0 {
                continue;
            }
            self.blend_pixel(x + i as i32, y, color, coverage);
        }
    }

    fn blend_vspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Color, u8),
    ) {
        if len <= 0 {
            return;
        }
        for i in 0..len as usize {
            let (color, coverage) = f(i);
            if coverage == 0 {
                continue;
            }
            self.blend_pixel(x, y + i as i32, color, coverage);
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, color: Color, coverages: &[u8]) {
        if coverages.is_empty() {
            return;
        }
        self.blend_hspan_with(x, y, coverages.len() as i32, |i| {
            let coverage = *coverages.get(i).unwrap_or(&0);
            debug_assert!(i < coverages.len(), "blend_solid_hspan: out of bounds coverage index");
            (color, coverage)
        });
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
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

    fn stamp_rgb_zero_alpha(&mut self, _x: i32, _y: i32, _color: Color) {}

    /// Unsafe escape hatch to raw buffer. Primitives should not use this.
    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}

// Minimal scaffold for an RGB565 rasterizer backend.
// Not wired into the system yet; provided for future migration.
extern crate alloc;



