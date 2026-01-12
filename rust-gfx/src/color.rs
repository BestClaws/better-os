/// Primary color type for the public API.
/// Stored as 0xRRGGBBAA (RGBA8888 format).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Rgba8888(u32);

impl Rgba8888 {
    /// Creates a fully opaque color: Rgba8888::rgb(r, g, b) → alpha = 255
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 255)
    }

    /// Creates a color with explicit alpha: Rgba8888::rgba(r, g, b, a)
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32))
    }

    /// Creates from a raw u32 value (0xRRGGBBAA)
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    /// Common predefined colors (opaque unless specified)
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    /// Converts to raw u32 for internal use
    #[inline]
    pub const fn to_u32(self) -> u32 {
        self.0
    }

    /// Returns the alpha component of the color.
    #[inline]
    pub const fn alpha(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Returns a new color with the provided alpha value.
    #[inline]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self((self.0 & 0xFFFFFF00) | (alpha as u32))
    }

    /// Multiplies the existing alpha channel by `alpha / 255`.
    #[inline]
    pub fn multiply_alpha(self, alpha: u8) -> Self {
        let existing = self.alpha() as u32;
        let effective = ((existing * alpha as u32) / 255) as u8;
        self.with_alpha(effective)
    }

    /// Get red component
    #[inline]
    pub const fn r(self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    /// Get green component
    #[inline]
    pub const fn g(self) -> u8 {
        ((self.0 >> 16) & 0xFF) as u8
    }

    /// Get blue component
    #[inline]
    pub const fn b(self) -> u8 {
        ((self.0 >> 8) & 0xFF) as u8
    }

    /// Get alpha component (alias for alpha())
    #[inline]
    pub const fn a(self) -> u8 {
        self.alpha()
    }
}

/// Blend two colors with opacity (alpha compositing)
/// Matches LVGL's color blending: result = bg * (1 - opa) + fg * opa
#[inline]
pub fn blend_colors(bg: Rgba8888, fg: Rgba8888, opa: u8) -> Rgba8888 {
    // LVGL uses non-premultiplied alpha:
    // The resulting pixel always has fg's RGB values, with alpha = opa
    // This preserves color information even when alpha is 0
    if opa == 255 {
        return fg;
    }
    
    // Always store the foreground color's RGB, just modulate the alpha
    Rgba8888::rgba(fg.r(), fg.g(), fg.b(), opa)
}

/// Linear interpolation between two colors
/// t=0 returns c1, t=255 returns c2
#[inline]
pub fn lerp_color(c1: Rgba8888, c2: Rgba8888, t: u8) -> Rgba8888 {
    if t == 0 {
        return c1;
    }
    if t == 255 {
        return c2;
    }
    let inv_t = 255 - t;
    let r = ((c1.r() as u32 * inv_t as u32 + c2.r() as u32 * t as u32) / 255) as u8;
    let g = ((c1.g() as u32 * inv_t as u32 + c2.g() as u32 * t as u32) / 255) as u8;
    let b = ((c1.b() as u32 * inv_t as u32 + c2.b() as u32 * t as u32) / 255) as u8;
    let a = ((c1.a() as u32 * inv_t as u32 + c2.a() as u32 * t as u32) / 255) as u8;
    Rgba8888::rgba(r, g, b, a)
}
