/// Core types for high-performance 2D graphics with fixed-point math optimizations
/// 
/// This module provides fundamental data types optimized for embedded systems:
/// - Fixed-point arithmetic for consistent performance
/// - SIMD-friendly data layouts
/// - Cache-optimized memory access patterns
/// - Zero-cost abstractions where possible

use core::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign};

/// Fixed-point number with 16.16 format for sub-pixel precision
/// Provides consistent performance across platforms without floating-point overhead
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed {
    pub raw: i32,
}

impl Fixed {
    /// Fixed-point scale factor (2^16)
    pub const SCALE: i32 = 65536;
    pub const ZERO: Fixed = Fixed { raw: 0 };
    pub const ONE: Fixed = Fixed { raw: Self::SCALE };
    pub const HALF: Fixed = Fixed { raw: Self::SCALE / 2 };
    
    /// Create from integer value
    #[inline(always)]
    pub const fn from_int(value: i32) -> Self {
        Self { raw: value * Self::SCALE }
    }
    
    /// Create from raw fixed-point value
    #[inline(always)]
    pub const fn from_raw(raw: i32) -> Self {
        Self { raw }
    }
    
    /// Create from f32 (for initialization, avoid in hot paths)
    #[inline]
    pub fn from_f32(value: f32) -> Self {
        Self { raw: (value * Self::SCALE as f32) as i32 }
    }
    
    /// Convert to integer (truncating)
    #[inline(always)]
    pub const fn to_int(self) -> i32 {
        self.raw / Self::SCALE
    }
    
    /// Convert to f32 (for debugging/display)
    #[inline]
    pub fn to_f32(self) -> f32 {
        self.raw as f32 / Self::SCALE as f32
    }
    
    /// Get fractional part (0.0 to 0.999...)
    #[inline(always)]
    pub const fn fract(self) -> Fixed {
        Self { raw: self.raw & (Self::SCALE - 1) }
    }
    
    /// Floor to integer
    #[inline(always)]
    pub const fn floor(self) -> Fixed {
        Self { raw: self.raw & !(Self::SCALE - 1) }
    }
    
    /// Ceiling to integer
    #[inline(always)]
    pub const fn ceil(self) -> Fixed {
        if self.raw & (Self::SCALE - 1) == 0 {
            self
        } else {
            Self { raw: (self.raw & !(Self::SCALE - 1)) + Self::SCALE }
        }
    }
    
    /// Absolute value
    #[inline(always)]
    pub const fn abs(self) -> Fixed {
        Self { raw: self.raw.abs() }
    }
    
    /// Fast multiplication (may overflow for large values)
    #[inline(always)]
    pub const fn mul_fast(self, other: Fixed) -> Fixed {
        Self { raw: (self.raw * other.raw) / Self::SCALE }
    }
    
    /// Safe multiplication with overflow protection
    #[inline]
    pub fn mul_safe(self, other: Fixed) -> Fixed {
        let result = (self.raw as i64 * other.raw as i64) / Self::SCALE as i64;
        Self { raw: result as i32 }
    }
    
    /// Square root approximation using Newton's method
    #[inline]
    pub fn sqrt(self) -> Fixed {
        if self.raw <= 0 { return Self::ZERO; }
        
        let mut x = self;
        let mut prev;
        
        // Newton's method: x = (x + n/x) / 2
        for _ in 0..8 { // 8 iterations for good precision
            prev = x;
            x = (x + self / x) / Fixed::from_int(2);
            if (x.raw - prev.raw).abs() < 2 { break; }
        }
        
        x
    }
}

impl Add for Fixed {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: Self) -> Self {
        Self { raw: self.raw + other.raw }
    }
}

impl Sub for Fixed {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        Self { raw: self.raw - other.raw }
    }
}

impl Mul for Fixed {
    type Output = Self;
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_safe(other)
    }
}

impl Div for Fixed {
    type Output = Self;
    #[inline]
    fn div(self, other: Self) -> Self {
        if other.raw == 0 { return Self::ZERO; }
        let result = (self.raw as i64 * Self::SCALE as i64) / other.raw as i64;
        Self { raw: result as i32 }
    }
}

impl AddAssign for Fixed {
    #[inline(always)]
    fn add_assign(&mut self, other: Self) {
        self.raw += other.raw;
    }
}

impl SubAssign for Fixed {
    #[inline(always)]
    fn sub_assign(&mut self, other: Self) {
        self.raw -= other.raw;
    }
}

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
    
    /// Linear interpolation between two colors
    #[inline]
    pub fn lerp(self, other: Self, t: Fixed) -> Self {
        if t.raw <= 0 { return self; }
        if t.raw >= Fixed::SCALE { return other; }
        
        let t_u8 = (t.raw >> 8) as u8; // Convert to 0-255 range
        let inv_t = 255 - t_u8;
        
        Self {
            r: ((self.r as u16 * inv_t as u16 + other.r as u16 * t_u8 as u16) / 255) as u8,
            g: ((self.g as u16 * inv_t as u16 + other.g as u16 * t_u8 as u16) / 255) as u8,
            b: ((self.b as u16 * inv_t as u16 + other.b as u16 * t_u8 as u16) / 255) as u8,
            a: ((self.a as u16 * inv_t as u16 + other.a as u16 * t_u8 as u16) / 255) as u8,
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
    pub fn from_fixed(x: Fixed, y: Fixed) -> Self {
        Self { x: x.to_int(), y: y.to_int() }
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
    pub top_left: Fixed,
    pub top_right: Fixed,
    pub bottom_right: Fixed,
    pub bottom_left: Fixed,
}

impl CornerRadii {
    /// Create corner radii with individual values
    #[inline]
    pub fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left: Fixed::from_f32(top_left),
            top_right: Fixed::from_f32(top_right),
            bottom_right: Fixed::from_f32(bottom_right),
            bottom_left: Fixed::from_f32(bottom_left),
        }
    }
    
    /// Create uniform corner radii
    #[inline(always)]
    pub fn uniform(radius: Fixed) -> Self {
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
        Self::uniform(Fixed::from_f32(radius))
    }
    
    /// Zero radii (sharp corners)
    #[inline(always)]
    pub const fn zero() -> Self {
        Self {
            top_left: Fixed::ZERO,
            top_right: Fixed::ZERO,
            bottom_right: Fixed::ZERO,
            bottom_left: Fixed::ZERO,
        }
    }
    
    /// Check if any corner has radius
    #[inline(always)]
    pub const fn has_radius(&self) -> bool {
        self.top_left.raw > 0
            || self.top_right.raw > 0
            || self.bottom_right.raw > 0
            || self.bottom_left.raw > 0
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
