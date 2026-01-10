/// Geometric types for rendering
/// Compatible with existing util::math::primitives but optimized for rendering

use micromath::F32Ext;

/// 2D point with integer coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// 2D point with floating-point coordinates (for sub-pixel accuracy)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

/// Rectangle defined by position and size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Point {
    /// Create a new point
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Origin point (0, 0)
    pub const fn zero() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Distance squared to another point (avoids sqrt)
    pub fn distance_sq(self, other: Point) -> i32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dx * dx + dy * dy
    }

    /// Manhattan distance to another point
    pub fn manhattan_distance(self, other: Point) -> i32 {
        (other.x - self.x).abs() + (other.y - self.y).abs()
    }
}

impl PointF {
    /// Create a new floating-point point
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Origin point (0.0, 0.0)
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Convert to integer point (truncating)
    pub fn to_point(self) -> Point {
        Point {
            x: self.x as i32,
            y: self.y as i32,
        }
    }

    /// Distance squared to another point (avoids sqrt)
    pub fn distance_sq(self, other: PointF) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dx * dx + dy * dy
    }

    /// Distance to another point
    pub fn distance(self, other: PointF) -> f32 {
        self.distance_sq(other).sqrt()
    }
}

impl Rect {
    /// Create a new rectangle
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Create rectangle from two corner points
    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x.max(p2.x) - x) as u32;
        let height = (p1.y.max(p2.y) - y) as u32;
        Self { x, y, width, height }
    }

    /// Create rectangle from coordinates
    pub fn from_coords(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let x = x1.min(x2);
        let y = y1.min(y2);
        let width = (x1.max(x2) - x) as u32;
        let height = (y1.max(y2) - y) as u32;
        Self { x, y, width, height }
    }

    /// Get top-left corner
    pub const fn top_left(self) -> Point {
        Point { x: self.x, y: self.y }
    }

    /// Get bottom-right corner
    pub const fn bottom_right(self) -> Point {
        Point {
            x: self.x + self.width as i32,
            y: self.y + self.height as i32,
        }
    }

    /// Get center point
    pub fn center(self) -> Point {
        Point {
            x: self.x + (self.width / 2) as i32,
            y: self.y + (self.height / 2) as i32,
        }
    }

    /// Check if rectangle contains a point
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width as i32
            && point.y >= self.y
            && point.y < self.y + self.height as i32
    }

    /// Check if rectangle intersects another rectangle
    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }

    /// Get intersection with another rectangle (returns None if no intersection)
    pub fn intersection(self, other: Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);

        if x1 < x2 && y1 < y2 {
            Some(Rect {
                x: x1,
                y: y1,
                width: (x2 - x1) as u32,
                height: (y2 - y1) as u32,
            })
        } else {
            None
        }
    }

    /// Get union with another rectangle
    pub fn union(self, other: Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width as i32).max(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).max(other.y + other.height as i32);

        Rect {
            x: x1,
            y: y1,
            width: (x2 - x1) as u32,
            height: (y2 - y1) as u32,
        }
    }

    /// Check if rectangle is empty (zero width or height)
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Get area in pixels
    pub const fn area(self) -> u32 {
        self.width * self.height
    }
}

// Conversions to/from existing util::math::primitives types
use crate::util::math::primitives::{Point as UtilPoint, Rect as UtilRect, Size as UtilSize};

impl From<UtilPoint> for Point {
    fn from(p: UtilPoint) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Point> for UtilPoint {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<UtilRect> for Rect {
    fn from(r: UtilRect) -> Self {
        Self {
            x: r.top_left.x,
            y: r.top_left.y,
            width: r.size.width,
            height: r.size.height,
        }
    }
}

impl From<Rect> for UtilRect {
    fn from(r: Rect) -> Self {
        Self {
            top_left: UtilPoint { x: r.x, y: r.y },
            size: UtilSize {
                width: r.width,
                height: r.height,
            },
        }
    }
}

impl From<Point> for PointF {
    fn from(p: Point) -> Self {
        Self {
            x: p.x as f32,
            y: p.y as f32,
        }
    }
}
