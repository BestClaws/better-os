use crate::colors::Color;


pub trait RasterTarget {
    /// Returns the width of the drawing surface measured in pixels.
    fn width(&self) -> u16;

    /// Returns the height of the drawing surface measured in pixels.
    fn height(&self) -> u16;

    // ========================================================================
    // HORIZONTAL SPAN OPERATIONS (hspan)
    // ========================================================================

    /// Writes a horizontal span of pixels using the provided source color and full coverage.
    ///
    /// Implementations may assume that every pixel is fully covered and can
    /// bypass blending when the source alpha is 255.
    ///
    /// # Parameters
    /// - `y`: Row coordinate (constant across the span)
    /// - `x_start`: Starting column coordinate
    /// - `color`: Source color applied to all pixels
    /// - `length`: Number of pixels to write
    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16);

    /// Blends a horizontal span that shares a single source color.
    ///
    /// `coverage` contains one entry per pixel in the span with values in the
    /// inclusive range `0..=255`. Implementations should premultiply the
    /// source color by the effective alpha (color alpha × coverage) before
    /// combining it with the destination.
    ///
    /// # Parameters
    /// - `y`: Row coordinate (constant across the span)
    /// - `x_start`: Starting column coordinate
    /// - `color`: Source color applied to all pixels
    /// - `coverage`: Per-pixel coverage values (0=transparent, 255=opaque)
    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]);

    /// Blends a horizontal span using per-pixel colors.
    ///
    /// `colors` and `coverage` describe the same span of pixels starting at
    /// `(x_start, y)` in row-major order. Implementations should process up to
    /// `min(colors.len(), coverage.len())` samples and apply coverage just as
    /// they do for [`blend_solid_hspan`].
    ///
    /// # Parameters
    /// - `y`: Row coordinate (constant across the span)
    /// - `x_start`: Starting column coordinate
    /// - `colors`: Per-pixel source colors
    /// - `coverage`: Per-pixel coverage values (0=transparent, 255=opaque)
    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]);

    // ========================================================================
    // VERTICAL SPAN OPERATIONS (vspan)
    // ========================================================================

    /// Writes a vertical span of pixels using the provided source color and full coverage.
    ///
    /// Vertical spans are useful for scanline rendering, column-major data structures,
    /// or when drawing vertical primitives like bars or dividers.
    ///
    /// # Parameters
    /// - `x`: Column coordinate (constant across the span)
    /// - `y_start`: Starting row coordinate
    /// - `color`: Source color applied to all pixels
    /// - `length`: Number of pixels to write
    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16);

    /// Blends a vertical span that shares a single source color.
    ///
    /// # Parameters
    /// - `x`: Column coordinate (constant across the span)
    /// - `y_start`: Starting row coordinate
    /// - `color`: Source color applied to all pixels
    /// - `coverage`: Per-pixel coverage values (0=transparent, 255=opaque)
    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]);

    /// Blends a vertical span using per-pixel colors.
    ///
    /// # Parameters
    /// - `x`: Column coordinate (constant across the span)
    /// - `y_start`: Starting row coordinate
    /// - `colors`: Per-pixel source colors
    /// - `coverage`: Per-pixel coverage values (0=transparent, 255=opaque)
    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]);

    // ========================================================================
    // RECTANGLE OPERATION
    // ========================================================================

    /// Fills an axis-aligned rectangle with a solid color and full coverage.
    ///
    /// This is an optimized primitive for drawing rectangular regions. Implementations
    /// may use tiling, SIMD operations, or hardware acceleration to fill large areas
    /// efficiently.
    ///
    /// # Parameters
    /// - `x`: Left edge coordinate
    /// - `y`: Top edge coordinate
    /// - `width`: Rectangle width in pixels
    /// - `height`: Rectangle height in pixels
    /// - `color`: Fill color
    ///
    /// # Default Implementation
    ///
    /// The default implementation decomposes the rectangle into horizontal spans,
    /// but custom implementations can override this for better performance.
    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        for row in y..y.saturating_add(height) {
            self.fill_solid_hspan(row, x, color, width);
        }
    }


}