/// ARGB8888 color type (32-bit with alpha channel)
/// Memory layout: AARRGGBB (alpha in high byte)
/// This matches SDL's pixel format for BMP output
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Argb8888(pub u32);

impl Argb8888 {
    /// Creates a fully opaque color
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Creates a color with explicit alpha
    #[inline]
    pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Creates from raw u32 value (AARRGGBB)
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    /// Converts to raw u32
    #[inline]
    pub const fn to_u32(self) -> u32 {
        self.0
    }

    /// Extract alpha component
    #[inline]
    pub const fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }

    /// Extract red component
    #[inline]
    pub const fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    /// Extract green component
    #[inline]
    pub const fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Extract blue component
    #[inline]
    pub const fn blue(self) -> u8 {
        self.0 as u8
    }

    /// Returns a new color with the provided alpha value
    #[inline]
    pub const fn with_alpha(self, alpha: u8) -> Self {
        Self((self.0 & 0x00FFFFFF) | ((alpha as u32) << 24))
    }

    /// Multiplies the existing alpha channel by `alpha / 255`
    #[inline]
    pub fn multiply_alpha(self, alpha: u8) -> Self {
        if alpha == 255 {
            return self;
        }
        let existing = self.alpha() as u32;
        let effective = ((existing * alpha as u32) / 255) as u8;
        self.with_alpha(effective)
    }

    /// Common predefined colors
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    pub const TRANSPARENT: Self = Self::argb(0, 0, 0, 0);

    /// Convert from LVGL-style color (RGB with separate alpha)
    #[inline]
    pub fn from_rgb_and_opa(r: u8, g: u8, b: u8, opa: u8) -> Self {
        Self::argb(opa, r, g, b)
    }
}

/// Linear interpolation between two colors
/// frac: 0 = color a, 255 = color b
#[inline]
pub fn lerp_color(a: Argb8888, b: Argb8888, frac: u8) -> Argb8888 {
    if frac == 0 {
        return a;
    }
    if frac == 255 {
        return b;
    }

    let inv = 255 - frac;

    let a_val = a.to_u32();
    let b_val = b.to_u32();

    let aa = ((a_val >> 24) & 0xFF) as u32;
    let ar = ((a_val >> 16) & 0xFF) as u32;
    let ag = ((a_val >> 8) & 0xFF) as u32;
    let ab = (a_val & 0xFF) as u32;

    let ba = ((b_val >> 24) & 0xFF) as u32;
    let br = ((b_val >> 16) & 0xFF) as u32;
    let bg = ((b_val >> 8) & 0xFF) as u32;
    let bb = (b_val & 0xFF) as u32;

    let ra = ((aa * inv as u32 + ba * frac as u32) / 255) as u32;
    let rr = ((ar * inv as u32 + br * frac as u32) / 255) as u32;
    let rg = ((ag * inv as u32 + bg * frac as u32) / 255) as u32;
    let rb = ((ab * inv as u32 + bb * frac as u32) / 255) as u32;

    Argb8888::from_u32((ra << 24) | (rr << 16) | (rg << 8) | rb)
}

/// Alpha blend: blend foreground onto background
/// opa: opacity of foreground (0 = fully transparent, 255 = fully opaque)
#[inline]
pub fn blend_colors(bg: Argb8888, fg: Argb8888, opa: u8) -> Argb8888 {
    if opa == 0 {
        return bg;
    }
    if opa == 255 && fg.alpha() == 255 {
        return fg;
    }

    // Calculate effective opacity (fg.alpha * opa / 255)
    let fg_alpha = fg.alpha() as u32;
    let effective_opa = ((fg_alpha * opa as u32) / 255) as u32;

    if effective_opa == 0 {
        return bg;
    }
    if effective_opa == 255 {
        return fg;
    }

    let inv = 255 - effective_opa;

    let bg_val = bg.to_u32();
    let fg_val = fg.to_u32();

    let br = ((bg_val >> 16) & 0xFF) as u32;
    let bg_g = ((bg_val >> 8) & 0xFF) as u32;
    let bb = (bg_val & 0xFF) as u32;

    let fr = ((fg_val >> 16) & 0xFF) as u32;
    let fg_g = ((fg_val >> 8) & 0xFF) as u32;
    let fb = (fg_val & 0xFF) as u32;

    let r = ((br * inv + fr * effective_opa) / 255) as u32;
    let g = ((bg_g * inv + fg_g * effective_opa) / 255) as u32;
    let b = ((bb * inv + fb * effective_opa) / 255) as u32;

    // Result alpha is blend of background and effective foreground alpha
    let ba = ((bg_val >> 24) & 0xFF) as u32;
    let a = ba + effective_opa - ((ba * effective_opa) / 255);

    Argb8888::from_u32((a << 24) | (r << 16) | (g << 8) | b)
}

// Keep old Rgba8888 as alias for compatibility (but deprecated)
#[deprecated(note = "Use Argb8888 instead")]
pub type Rgba8888 = Argb8888;
