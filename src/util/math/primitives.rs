use core::ops::{Add, Sub};
use defmt::Format;
use fixed::types::extra::U16;
use fixed::FixedI32;
use micromath::F32Ext;

#[derive(Copy, Clone, Debug, Format)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub fn add(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }

    pub fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }

    pub fn dot(self, rhs: Vec3) -> f32 {
        self.0 * rhs.0 + self.1 * rhs.1 + self.2 * rhs.2
    }

    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3(
            self.1 * rhs.2 - self.2 * rhs.1,
            self.2 * rhs.0 - self.0 * rhs.2,
            self.0 * rhs.1 - self.1 * rhs.0,
        )
    }

    pub fn scale(self, s: f32) -> Vec3 {
        Vec3(self.0 * s, self.1 * s, self.2 * s)
    }

    pub fn normalize(self) -> Vec3 {
        let mag = (self.0 * self.0 + self.1 * self.1 + self.2 * self.2).sqrt();
        if mag > 0.0 {
            self.scale(1.0 / mag)
        } else {
            self
        }
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.0 * self.0 + self.1 * self.1 + self.2 * self.2
    }
}

/// A quaternion representing rotation
#[derive(Copy, Clone, Debug, Format)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    /// Rotates a vector by this quaternion
    pub fn rotate_vector(self, v: Vec3) -> Vec3 {
        let u = Vec3(self.x, self.y, self.z);
        let s = self.w;

        let uv = u.cross(v);
        let uuv = u.cross(uv);

        v.add(uv.scale(2.0 * s)).add(uuv.scale(2.0))
    }

    pub fn magnitude(&self) -> f32 {
        libm::sqrt((self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z) as f64)
            as f32
    }

    /// Normalizes the quaternion to have magnitude 1.
    ///
    /// Normalization is important because:
    /// 1. Only unit quaternions (magnitude = 1) represent pure rotations
    /// 2. Prevents scaling effects from accumulating during calculations
    /// 3. Maintains numerical stability in orientation tracking
    ///
    /// The process divides each component by the quaternion's magnitude.
    pub fn normalize(&self) -> Self {
        let m = self.magnitude();
        if m > 1.0e-6 {
            let inv = 1.0 / m;
            Self {
                w: self.w * inv,
                x: self.x * inv,
                y: self.y * inv,
                z: self.z * inv,
            }
        } else {
            Self::identity()
        }
    }

    /// Returns the identity quaternion (no rotation).
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Returns the conjugate (inverse for unit quaternions).
    pub fn conjugate(&self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Multiplies two quaternions using Hamilton product rules.
    pub fn mul(&self, rhs: &Self) -> Self {
        Self {
            w: self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            x: self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            y: self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            z: self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        }
    }
}

/// 2D point with sub-pixel precision using fixed-point coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Create new point
    #[inline(always)]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero point
    #[inline(always)]
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Create from fixed-point coordinates
    #[inline(always)]
    pub fn from_fixed(x: FixedI32<U16>, y: FixedI32<U16>) -> Self {
        Self {
            x: x.to_num(),
            y: y.to_num(),
        }
    }

    /// Distance squared to another point (avoids sqrt for performance)
    #[inline]
    pub fn distance_squared(self, other: Point) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Manhattan distance (faster than Euclidean)
    #[inline(always)]
    pub fn manhattan_distance(self, other: Point) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

impl Add for Point {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

/// 2D size with width and height
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    /// Create new size
    #[inline(always)]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Zero size
    #[inline(always)]
    pub const fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    /// Area of the size
    #[inline(always)]
    pub const fn area(self) -> u32 {
        self.width * self.height
    }
}

/// Rectangle defined by top-left point and size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub struct Rect {
    pub top_left: Point,
    pub size: Size,
}

impl Rect {
    /// Create new rectangle
    #[inline(always)]
    pub const fn new(top_left: Point, size: Size) -> Self {
        Self { top_left, size }
    }

    /// Create rectangle from coordinates
    #[inline(always)]
    pub const fn from_coords(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            top_left: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Create rectangle from corner points
    #[inline(always)]
    pub const fn with_corners(top_left: Point, bottom_right: Point) -> Self {
        let width = (bottom_right.x - top_left.x + 1) as u32;
        let height = (bottom_right.y - top_left.y + 1) as u32;
        Self {
            top_left,
            size: Size::new(width, height),
        }
    }

    /// Right edge coordinate
    #[inline(always)]
    pub const fn right(self) -> i32 {
        self.top_left.x + self.size.width as i32 - 1
    }

    /// Bottom edge coordinate
    #[inline(always)]
    pub const fn bottom(self) -> i32 {
        self.top_left.y + self.size.height as i32 - 1
    }

    /// Center point
    #[inline(always)]
    pub const fn center(self) -> Point {
        Point::new(
            self.top_left.x + self.size.width as i32 / 2,
            self.top_left.y + self.size.height as i32 / 2,
        )
    }

    /// Check if point is inside rectangle
    #[inline(always)]
    pub const fn contains_point(self, point: Point) -> bool {
        point.x >= self.top_left.x
            && point.y >= self.top_left.y
            && point.x <= self.right()
            && point.y <= self.bottom()
    }

    /// Check if rectangle intersects with another
    #[inline]
    pub const fn intersects(self, other: Rect) -> bool {
        self.top_left.x <= other.right()
            && self.right() >= other.top_left.x
            && self.top_left.y <= other.bottom()
            && self.bottom() >= other.top_left.y
    }

    /// Intersection with another rectangle
    #[inline]
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let left = self.top_left.x.max(other.top_left.x);
        let top = self.top_left.y.max(other.top_left.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if left <= right && top <= bottom {
            Some(Rect::from_coords(
                left,
                top,
                (right - left + 1) as u32,
                (bottom - top + 1) as u32,
            ))
        } else {
            None
        }
    }
}
