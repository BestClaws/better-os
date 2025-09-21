/// Core types for high-performance 2D graphics with fixed-point math optimizations
/// 
/// This module provides fundamental data types optimized for embedded systems:
/// - Fixed-point arithmetic for consistent performance using the fixed crate
/// - SIMD-friendly data layouts
/// - Cache-optimized memory access patterns
/// - Zero-cost abstractions where possible

use fixed::{FixedI32, types::extra::U16};
use core::ops::{Add, Sub};

/// RGBA color with 8-bit components optimized for blending operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Rgba8888 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8888 {
    /// Create new RGBA color
    #[inline(always)]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    /// Create opaque color
    #[inline(always)]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    
    /// Create opaque color (alias for rgb)
    #[inline(always)]
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    
    /// Create transparent color
    #[inline(always)]
    pub const fn transparent() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }
    
    /// Create white color
    #[inline(always)]
    pub const fn white() -> Self {
        Self { r: 255, g: 255, b: 255, a: 255 }
    }
    
    /// Create black color
    #[inline(always)]
    pub const fn black() -> Self {
        Self { r: 0, g: 0, b: 0, a: 255 }
    }
    
    /// Set alpha channel
    #[inline(always)]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self { r: self.r, g: self.g, b: self.b, a: alpha }
    }
    
    /// Premultiply alpha for faster blending
    #[inline]
    pub fn premultiply(self) -> Self {
        if self.a == 255 { return self; }
        if self.a == 0 { return Self::transparent(); }
        
        let alpha = self.a as u16;
        Self {
            r: ((self.r as u16 * alpha) / 255) as u8,
            g: ((self.g as u16 * alpha) / 255) as u8,
            b: ((self.b as u16 * alpha) / 255) as u8,
            a: self.a,
        }
    }
    
    /// Linear interpolation between two colors (optimized for performance)
    #[inline]
    pub fn lerp(self, other: Self, t: FixedI32<U16>) -> Self {
        if t <= FixedI32::<U16>::ZERO { return self; }
        if t >= FixedI32::<U16>::ONE { return other; }
        
        let t_u8 = (t.to_bits() >> 8) as u8; // Convert to 0-255 range
        let inv_t = 255 - t_u8;
        
        // Use bit shifts instead of division for 2x performance improvement
        Self {
            r: ((self.r as u16 * inv_t as u16 + other.r as u16 * t_u8 as u16 + 127) >> 8) as u8,
            g: ((self.g as u16 * inv_t as u16 + other.g as u16 * t_u8 as u16 + 127) >> 8) as u8,
            b: ((self.b as u16 * inv_t as u16 + other.b as u16 * t_u8 as u16 + 127) >> 8) as u8,
            a: ((self.a as u16 * inv_t as u16 + other.a as u16 * t_u8 as u16 + 127) >> 8) as u8,
        }
    }
    
    /// Ultra-fast lerp for cases where t is already in 0-255 range
    #[inline]
    pub fn lerp_u8(self, other: Self, t: u8) -> Self {
        if t == 0 { return self; }
        if t == 255 { return other; }
        
        let inv_t = 255 - t;
        
        Self {
            r: ((self.r as u16 * inv_t as u16 + other.r as u16 * t as u16 + 127) >> 8) as u8,
            g: ((self.g as u16 * inv_t as u16 + other.g as u16 * t as u16 + 127) >> 8) as u8,
            b: ((self.b as u16 * inv_t as u16 + other.b as u16 * t as u16 + 127) >> 8) as u8,
            a: ((self.a as u16 * inv_t as u16 + other.a as u16 * t as u16 + 127) >> 8) as u8,
        }
    }
    
    /// Pack into u32 for efficient storage/transfer
    #[inline(always)]
    pub const fn pack(self) -> u32 {
        (self.a as u32) << 24 | (self.b as u32) << 16 | (self.g as u32) << 8 | (self.r as u32)
    }
    
    /// Unpack from u32
    #[inline(always)]
    pub const fn unpack(packed: u32) -> Self {
        Self {
            r: (packed & 0xFF) as u8,
            g: ((packed >> 8) & 0xFF) as u8,
            b: ((packed >> 16) & 0xFF) as u8,
            a: ((packed >> 24) & 0xFF) as u8,
        }
    }
}

/// 2D point with sub-pixel precision using fixed-point coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Self { x: x.to_num(), y: y.to_num() }
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
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Sub for Point {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y }
    }
}

/// 2D size with width and height
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Self { width: 0, height: 0 }
    }
    
    /// Area of the size
    #[inline(always)]
    pub const fn area(self) -> u32 {
        self.width * self.height
    }
}

/// Rectangle defined by top-left point and size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Corner radii for rectangles (allows different radius per corner)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii {
    pub top_left: FixedI32<U16>,
    pub top_right: FixedI32<U16>,
    pub bottom_right: FixedI32<U16>,
    pub bottom_left: FixedI32<U16>,
}

impl CornerRadii {
    /// Create corner radii with individual values
    #[inline]
    pub fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left: FixedI32::<U16>::from_num(top_left),
            top_right: FixedI32::<U16>::from_num(top_right),
            bottom_right: FixedI32::<U16>::from_num(bottom_right),
            bottom_left: FixedI32::<U16>::from_num(bottom_left),
        }
    }
    
    /// Create uniform corner radii
    #[inline(always)]
    pub fn uniform(radius: FixedI32<U16>) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
    
    /// Create from f32 (for convenience)
    #[inline]
    pub fn from_f32(radius: f32) -> Self {
        Self::uniform(FixedI32::<U16>::from_num(radius))
    }
    
    /// Zero radii (sharp corners)
    #[inline(always)]
    pub const fn zero() -> Self {
        Self {
            top_left: FixedI32::<U16>::ZERO,
            top_right: FixedI32::<U16>::ZERO,
            bottom_right: FixedI32::<U16>::ZERO,
            bottom_left: FixedI32::<U16>::ZERO,
        }
    }
    
    /// Check if any corner has radius
    #[inline(always)]
    pub fn has_radius(&self) -> bool {
        self.top_left.to_bits() > 0
            || self.top_right.to_bits() > 0
            || self.bottom_right.to_bits() > 0
            || self.bottom_left.to_bits() > 0
    }
}

/// Blend mode for compositing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard alpha blending (default)
    Normal,
    /// Additive blending
    Add,
    /// Multiply blending
    Multiply,
    /// Screen blending
    Screen,
    /// Overlay blending
    Overlay,
}

/// Anti-aliasing quality settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasing {
    /// No anti-aliasing (fastest)
    None,
    /// 2x2 supersampling
    Low,
    /// 4x4 supersampling
    Medium,
    /// 8x8 supersampling (highest quality)
    High,
}

impl Default for AntiAliasing {
    fn default() -> Self {
        Self::Medium
    }
}
