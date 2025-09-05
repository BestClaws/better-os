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

    /// High-performance alpha blending between two RGB565 colors.
    /// 
    /// This function uses optimized integer arithmetic to perform alpha blending
    /// between a source color and a background color, achieving maximum performance
    /// while maintaining visual quality.
    /// 
    /// Key optimizations:
    /// - Fast path for common alpha values (0, 255)
    /// - Optimized integer arithmetic to avoid floating point operations
    /// - Efficient bit operations for color component extraction and packing
    /// - Minimal branching for better performance
    /// 
    /// # Arguments
    /// * `bg` - Background color to blend with
    /// * `alpha_u8` - Alpha value (0-255, where 255 is fully opaque)
    /// 
    /// # Returns
    /// Blended RGB565 color
    pub fn blend_over(self, bg: Rgb565, alpha_u8: u8) -> Rgb565 {
        // Fast path for common alpha values
        if alpha_u8 == 255 {
            return self;
        }
        if alpha_u8 == 0 {
            return bg;
        }
        
        // Extract color components using optimized bit operations
        let src_r = ((self.0 >> 11) & 0x1F) as u32;
        let src_g = ((self.0 >> 5) & 0x3F) as u32;
        let src_b = (self.0 & 0x1F) as u32;
        
        let bg_r = ((bg.0 >> 11) & 0x1F) as u32;
        let bg_g = ((bg.0 >> 5) & 0x3F) as u32;
        let bg_b = (bg.0 & 0x1F) as u32;
        
        // Perform alpha blending with optimized arithmetic
        let alpha = alpha_u8 as u32;
        let alpha_inv = 255 - alpha;
        let r = (src_r * alpha + bg_r * alpha_inv + 127) / 255;
        let g = (src_g * alpha + bg_g * alpha_inv + 127) / 255;
        let b = (src_b * alpha + bg_b * alpha_inv + 127) / 255;
        
        // Pack back into RGB565 format using optimized bit operations
        let packed = (((r & 0x1F) as u16) << 11) | (((g & 0x3F) as u16) << 5) | ((b & 0x1F) as u16);
        Rgb565(packed)
    }
    
    /// Ultra-fast alpha blending for common alpha values.
    /// 
    /// This function provides optimized blending for common alpha values
    /// (0, 128, 255) using lookup tables and bit operations for maximum performance.
    /// 
    /// # Arguments
    /// * `bg` - Background color to blend with
    /// * `alpha_u8` - Alpha value (0-255, where 255 is fully opaque)
    /// 
    /// # Returns
    /// Blended RGB565 color
    pub fn blend_over_fast(self, bg: Rgb565, alpha_u8: u8) -> Rgb565 {
        match alpha_u8 {
            0 => bg,
            255 => self,
            128 => {
                // 50% blending - use bit operations for maximum speed
                let src_r = (self.0 >> 11) & 0x1F;
                let src_g = (self.0 >> 5) & 0x3F;
                let src_b = self.0 & 0x1F;
                
                let bg_r = (bg.0 >> 11) & 0x1F;
                let bg_g = (bg.0 >> 5) & 0x3F;
                let bg_b = bg.0 & 0x1F;
                
                let r = (src_r + bg_r) >> 1;
                let g = (src_g + bg_g) >> 1;
                let b = (src_b + bg_b) >> 1;
                
                Rgb565((r << 11) | (g << 5) | b)
            }
            _ => self.blend_over(bg, alpha_u8),
        }
    }
    
    /// High-performance color interpolation using fixed-point arithmetic.
    /// 
    /// This function performs linear interpolation between two RGB565 colors
    /// using 16.16 fixed-point arithmetic for maximum performance.
    /// 
    /// # Arguments
    /// * `other` - Other color to interpolate with
    /// * `t_fixed` - Interpolation parameter in 16.16 fixed point format
    /// 
    /// # Returns
    /// Interpolated RGB565 color
    pub fn interpolate_fixed(self, other: Rgb565, t_fixed: i32) -> Rgb565 {
        // Extract color components with 8-bit precision for interpolation
        let self_r = ((self.0 >> 11) & 0x1F) as i32;
        let self_g = ((self.0 >> 5) & 0x3F) as i32;
        let self_b = (self.0 & 0x1F) as i32;
        
        let other_r = ((other.0 >> 11) & 0x1F) as i32;
        let other_g = ((other.0 >> 5) & 0x3F) as i32;
        let other_b = (other.0 & 0x1F) as i32;
        
        // Interpolate using fixed-point arithmetic (t_fixed is 16.16 format)
        let result_r = self_r + (((other_r - self_r) * t_fixed) >> 16);
        let result_g = self_g + (((other_g - self_g) * t_fixed) >> 16);
        let result_b = self_b + (((other_b - self_b) * t_fixed) >> 16);
        
        // Clamp to valid ranges and pack back into RGB565
        let r = (result_r.clamp(0, 31) as u16) << 11;
        let g = (result_g.clamp(0, 63) as u16) << 5;
        let b = (result_b.clamp(0, 31) as u16);
        
        Rgb565(r | g | b)
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
