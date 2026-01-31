/// Primary color type for the public API.
/// Stored as 0xRRGGBBAA (RGBA8888 format).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Color(u32);

impl Color {
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
pub fn blend_colors(bg: Color, fg: Color, opa: u8) -> Color {
    // LVGL non-premultiplied alpha: ALWAYS store foreground color with foreground alpha
    // Even at opa=0, preserve fg RGB (this is critical for LVGL compatibility)

    if opa >= 255 {
        return fg;
    }

    // Match LVGL: keep existing pixel if it already has coverage, otherwise stamp fg with 0 alpha
    if opa == 0 {
        if bg.a() == 0 {
            return Color::rgba(fg.r(), fg.g(), fg.b(), 0);
        }
        return bg;
    }

    // If background is fully transparent, just use fg color with opa (no blending)
    let bg_alpha = bg.a();
    if bg_alpha == 0 {
        return Color::rgba(fg.r(), fg.g(), fg.b(), opa);
    }

    // LVGL Porter-Duff OVER compositing when both colors have alpha
    // https://en.wikipedia.org/wiki/Alpha_compositing#Analytical_derivation_of_the_over_operator

    // First calculate composited alpha using LVGL's LV_OPA_MIX2 (>> 8, not udiv255!)
    let inv_fg_a = 255 - opa;
    let inv_bg_a = 255 - bg_alpha;
    let result_alpha = 255 - ((inv_fg_a as u32 * inv_bg_a as u32) >> 8) as u8;

    // Calculate ratio for RGB blending using LVGL's exact formula
    // ratio_saved = (fg.alpha * 255) / res_alpha_saved (simple integer division!)
    let mut ratio = if result_alpha > 0 {
        ((opa as u32 * 255) / result_alpha as u32).min(255) as u8
    } else {
        0
    };

    if bg_alpha <= 32 && ratio >= 253 {
        ratio = 255;
    }

    let inv_ratio = 255 - ratio;
    let r = udiv255(fg.r() as u32 * ratio as u32 + bg.r() as u32 * inv_ratio as u32);
    let g = udiv255(fg.g() as u32 * ratio as u32 + bg.g() as u32 * inv_ratio as u32);
    let b = udiv255(fg.b() as u32 * ratio as u32 + bg.b() as u32 * inv_ratio as u32);

    Color::rgba(r, g, b, result_alpha)
}

/// Fast divide by 255 using LVGL's method
/// LV_UDIV255(x) = ((x * 0x8081) >> 23)
#[inline]
fn udiv255(x: u32) -> u8 {
    ((x * 0x8081) >> 23) as u8
}

/// Linear interpolation between two colors
/// t=0 returns c1, t=255 returns c2
/// Matches LVGL's exact formula: LV_UDIV255(c2 * t + c1 * (255-t))
#[inline]
pub fn lerp_color(c1: Color, c2: Color, t: u8) -> Color {
    if t == 0 {
        return c1;
    }
    if t == 255 {
        return c2;
    }
    let inv_t = 255 - t;
    // LVGL does: color2 * mix + color1 * (255-mix), then udiv255
    let r = udiv255(c2.r() as u32 * t as u32 + c1.r() as u32 * inv_t as u32);
    let g = udiv255(c2.g() as u32 * t as u32 + c1.g() as u32 * inv_t as u32);
    let b = udiv255(c2.b() as u32 * t as u32 + c1.b() as u32 * inv_t as u32);
    let a = udiv255(c2.a() as u32 * t as u32 + c1.a() as u32 * inv_t as u32);
    Color::rgba(r, g, b, a)
}
