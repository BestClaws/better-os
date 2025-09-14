use core::cmp::{max, min};
use defmt::Format;

// Geometry primitives
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
}

// Extents in pixels
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

// Axis-aligned rectangle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }

    pub fn with_corners(a: Point, b: Point) -> Self {
        let left = min(a.x, b.x);
        let top = min(a.y, b.y);
        let right = max(a.x, b.x);
        let bottom = max(a.y, b.y);
        Self {
            top_left: Point::new(left, top),
            size: Size::new((right - left + 1) as u32, (bottom - top + 1) as u32),
        }
    }

    pub fn right(&self) -> i32 {
        self.top_left.x + self.size.width as i32 - 1
    }
    pub fn bottom(&self) -> i32 {
        self.top_left.y + self.size.height as i32 - 1
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.right() < other.top_left.x
            || other.right() < self.top_left.x
            || self.bottom() < other.top_left.y
            || other.bottom() < self.top_left.y)
    }

    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        let left = max(self.top_left.x, other.top_left.x);
        let top = max(self.top_left.y, other.top_left.y);
        let right = min(self.right(), other.right());
        let bottom = min(self.bottom(), other.bottom());
        Some(Rect::with_corners(
            Point::new(left, top),
            Point::new(right, bottom),
        ))
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let left = min(self.top_left.x, other.top_left.x);
        let top = min(self.top_left.y, other.top_left.y);
        let right = max(self.right(), other.right());
        let bottom = max(self.bottom(), other.bottom());
        Rect::with_corners(Point::new(left, top), Point::new(right, bottom))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba8888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8888 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
    #[inline(always)]
    pub const fn with_alpha(self, a: u8) -> Self { Self { a, ..self } }

    /// Multiply current alpha by factor (0..=255), returning a new color.
    #[inline(always)]
    pub fn mul_alpha_u8(self, factor: u8) -> Self {
        if factor == 255 { return self; }
        if factor == 0 { return Self { a: 0, ..self }; }
        let a = ((self.a as u16 * factor as u16 + 127) / 255) as u8;
        Self { a, ..self }
    }

    /// Alpha blend this color over `dst`, returning the composited color.
    /// Uses straight alpha with 8-bit integer math.
    #[inline(always)]
    pub fn blend_over(self, dst: Rgba8888) -> Rgba8888 {
        if self.a == 255 { return Self { a: 255, ..self }; }
        if self.a == 0 { return dst; }
        let a = self.a as u32;
        let ia = 255 - a;
        let r = (self.r as u32 * a + dst.r as u32 * ia + 127) / 255;
        let g = (self.g as u32 * a + dst.g as u32 * ia + 127) / 255;
        let b = (self.b as u32 * a + dst.b as u32 * ia + 127) / 255;
        Rgba8888 { r: r as u8, g: g as u8, b: b as u8, a: 255 }
    }
}

