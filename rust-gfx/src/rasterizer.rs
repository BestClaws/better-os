use crate::color::Rgba8888;
use crate::surface::{self, ClampedRect, ClampedSpan};

/// Common interface implemented by every rasterizer backend.
///
/// The trait supplies a few convenience helpers (such as bounds checks and
/// clipping utilities) so that backends can share consistent behaviour
/// without re-implementing boilerplate in each method.
pub trait Rasterizer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn buffer_mut(&mut self) -> &mut [u8];

    /// Clear entire surface with a solid color.
    fn clear(&mut self, color: Rgba8888);

    /// Width of the rasterizer in signed coordinates.
    #[inline]
    fn width_i32(&self) -> i32 {
        self.width() as i32
    }

    /// Height of the rasterizer in signed coordinates.
    #[inline]
    fn height_i32(&self) -> i32 {
        self.height() as i32
    }

    /// Returns true when the given pixel is inside the surface bounds.
    #[inline]
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width_i32() && y < self.height_i32()
    }

    /// Clamp a horizontal span to the surface width.
    #[inline]
    fn clamp_horizontal_span(&self, start: i32, len: i32) -> Option<ClampedSpan> {
        surface::clamp_span(start, len, 0, self.width_i32())
    }

    /// Clamp a vertical span to the surface height.
    #[inline]
    fn clamp_vertical_span(&self, start: i32, len: i32) -> Option<ClampedSpan> {
        surface::clamp_span(start, len, 0, self.height_i32())
    }

    /// Clamp a rectangle defined by origin and size to the surface bounds.
    #[inline]
    fn clamp_rect_from_size(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ClampedRect> {
        surface::clamp_rect_from_size(x, y, w, h, self.width_i32(), self.height_i32())
    }

    /// New high-level, pixel-format-agnostic APIs with default implementations.
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        // Default: bounds check only
        if !self.in_bounds(x, y) {
            return;
        }
        if coverage == 0 {
            return;
        }
        // Subclasses should override this with format-specific implementation
        unimplemented!("blend_pixel must be implemented by the rasterizer backend");
    }

    fn blend_hspan(&mut self, x: i32, y: i32, colors: &[Rgba8888], coverages: Option<&[u8]>) {
        if colors.is_empty() {
            return;
        }
        let len = colors.len() as i32;
        self.blend_hspan_with(x, y, len, |i| {
            let coverage = coverages.and_then(|c| c.get(i)).copied().unwrap_or(255);
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
        if len <= 0 || y < 0 || y >= self.height_i32() {
            return;
        }
        if let Some(span) = self.clamp_horizontal_span(x, len) {
            for i in 0..span.len() {
                let (color, coverage) = f(i + span.skip);
                if coverage == 0 {
                    continue;
                }
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
        if len <= 0 || x < 0 || x >= self.width_i32() {
            return;
        }
        if let Some(span) = self.clamp_vertical_span(y, len) {
            for i in 0..span.len() {
                let (color, coverage) = f(i + span.skip);
                if coverage == 0 {
                    continue;
                }
                self.blend_pixel(x, span.start + i as i32, color, coverage);
            }
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, color: Rgba8888, coverages: &[u8]) {
        if coverages.is_empty() {
            return;
        }
        self.blend_hspan_with(x, y, coverages.len() as i32, |i| {
            let coverage = *coverages.get(i).unwrap_or(&0);
            debug_assert!(
                i < coverages.len(),
                "blend_solid_hspan: out of bounds coverage index"
            );
            (color, coverage)
        });
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        // Default implementation using blend_hspan_with for each row.
        if let Some(region) = self.clamp_rect_from_size(x, y, w, h) {
            let span_width = region.width();
            for py in region.min_y..region.max_y {
                self.blend_hspan_with(region.min_x, py, span_width, |_| (color, 255));
            }
        }
    }

    fn stamp_rgb_zero_alpha(&mut self, _x: i32, _y: i32, _color: Rgba8888) {}

    /// Unsafe escape hatch to raw buffer. Primitives should not use this.
    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }
}
