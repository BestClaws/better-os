use core::cmp::{max, min};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub struct Rgb565(pub u16);

impl Rgb565 {
    pub const BLACK: Self = Self(0x0000);
    pub const WHITE: Self = Self(0xFFFF);

    pub const fn from_rgb(r8: u8, g8: u8, b8: u8) -> Self {
        let r5: u16 = ((r8 as u16) >> 3) & 0x1F;
        let g6: u16 = ((g8 as u16) >> 2) & 0x3F;
        let b5: u16 = ((b8 as u16) >> 3) & 0x1F;
        let packed: u16 = (r5 << 11) | (g6 << 5) | b5;
        Self(packed)
    }

    pub const fn into_storage(self) -> u16 {
        self.0
    }

    pub fn blend_over(self, bg: Rgb565, alpha_u8: u8) -> Rgb565 {
        let a = alpha_u8 as u32;
        if a == 255 {
            return self;
        }
        if a == 0 {
            return bg;
        }
        let sr = ((self.0 >> 11) & 0x1F) as u32;
        let sg = ((self.0 >> 5) & 0x3F) as u32;
        let sb = (self.0 & 0x1F) as u32;
        let br = ((bg.0 >> 11) & 0x1F) as u32;
        let bgc = ((bg.0 >> 5) & 0x3F) as u32;
        let bb = (bg.0 & 0x1F) as u32;
        let r = (sr * a + br * (255 - a) + 127) / 255;
        let g = (sg * a + bgc * (255 - a) + 127) / 255;
        let b = (sb * a + bb * (255 - a) + 127) / 255;
        let packed: u16 =
            (((r & 0x1F) as u16) << 11) | (((g & 0x3F) as u16) << 5) | ((b & 0x1F) as u16);
        Rgb565(packed)
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
    pub fn to_rgb565(self) -> Rgb565 {
        Rgb565::from_rgb(self.r, self.g, self.b)
    }
}
