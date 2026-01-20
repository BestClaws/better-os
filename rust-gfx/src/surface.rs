//! Helpers for raster surfaces such as clipping spans and rectangles.
//!
//! The rasterizer implementations in the crate frequently need to clamp
//! drawing requests to the actual surface bounds.  Centralising the logic
//! keeps the behaviour consistent and avoids a proliferation of ad-hoc
//! helpers in individual backends.

/// Result of clamping a line segment (span) against a half-open interval.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ClampedSpan {
    /// Inclusive starting coordinate after clamping.
    pub start: i32,
    /// Exclusive end coordinate after clamping.
    pub end: i32,
    /// Number of leading elements skipped from the original span.
    pub skip: usize,
}

impl ClampedSpan {
    /// Length of the clamped span in integer coordinates.
    #[inline]
    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    /// Returns `true` when the span contains no pixels.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// Result of clamping an axis-aligned rectangle against surface bounds.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ClampedRect {
    /// Inclusive minimum X coordinate.
    pub min_x: i32,
    /// Inclusive minimum Y coordinate.
    pub min_y: i32,
    /// Exclusive maximum X coordinate.
    pub max_x: i32,
    /// Exclusive maximum Y coordinate.
    pub max_y: i32,
}

impl ClampedRect {
    /// Width of the clamped rectangle in pixels.
    #[inline]
    pub fn width(&self) -> i32 {
        self.max_x - self.min_x
    }

    /// Height of the clamped rectangle in pixels.
    #[inline]
    pub fn height(&self) -> i32 {
        self.max_y - self.min_y
    }

    /// Returns `true` when the rectangle has no area.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width() <= 0 || self.height() <= 0
    }
}

fn saturating_end(start: i32, len: i32) -> i32 {
    match len {
        i32::MIN => start,
        _ => start.saturating_add(len),
    }
}

/// Clamp a span defined by a start coordinate and length to the given bounds.
///
/// The returned span uses half-open coordinates `[start, end)` so that it can be
/// fed directly into iterator ranges.
#[inline]
pub fn clamp_span(start: i32, len: i32, min: i32, max: i32) -> Option<ClampedSpan> {
    if len <= 0 || min >= max {
        return None;
    }

    let requested_end = saturating_end(start, len);
    let clamped_start = start.clamp(min, max);
    let clamped_end = requested_end.clamp(min, max);

    if clamped_start >= clamped_end {
        return None;
    }

    let skip = clamped_start.saturating_sub(start) as usize;
    Some(ClampedSpan {
        start: clamped_start,
        end: clamped_end,
        skip,
    })
}

/// Clamp a rectangle expressed as half-open bounds to the given limits.
#[inline]
pub fn clamp_rect(
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    width: i32,
    height: i32,
) -> Option<ClampedRect> {
    if min_x >= max_x || min_y >= max_y || width <= 0 || height <= 0 {
        return None;
    }

    let clamped_min_x = min_x.clamp(0, width);
    let clamped_min_y = min_y.clamp(0, height);
    let clamped_max_x = max_x.clamp(0, width);
    let clamped_max_y = max_y.clamp(0, height);

    if clamped_min_x >= clamped_max_x || clamped_min_y >= clamped_max_y {
        return None;
    }

    Some(ClampedRect {
        min_x: clamped_min_x,
        min_y: clamped_min_y,
        max_x: clamped_max_x,
        max_y: clamped_max_y,
    })
}

/// Clamp a rectangle defined by origin plus size to the given limits.
#[inline]
pub fn clamp_rect_from_size(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    limit_width: i32,
    limit_height: i32,
) -> Option<ClampedRect> {
    if width <= 0 || height <= 0 {
        return None;
    }

    let max_x = saturating_end(x, width);
    let max_y = saturating_end(y, height);
    clamp_rect(x, y, max_x, max_y, limit_width, limit_height)
}