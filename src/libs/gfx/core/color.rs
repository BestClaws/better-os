/// Core color types for the rendering system
/// Based on LVGL color model with Rust ergonomics

/// RGB color (8 bits per channel)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// RGBA color with alpha channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorAlpha {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Opacity value (0 = transparent, 255 = opaque)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Opacity(u8);

/// HSV color representation
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: u16,  // 0-360 degrees
    pub s: u8,   // 0-100 percent
    pub v: u8,   // 0-100 percent
}

/// Supported color formats for rendering targets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    /// 8-bit grayscale
    L8,
    /// 8-bit alpha only
    A8,
    /// 16-bit RGB (5-6-5)
    Rgb565,
    /// 24-bit RGB
    Rgb888,
    /// 32-bit ARGB
    Argb8888,
    /// 32-bit RGB (no alpha)
    Xrgb8888,
}

impl Color {
    /// Create a new RGB color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Create color from hex value (0xRRGGBB)
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// Create color from HSV
    pub fn from_hsv(h: u16, s: u8, v: u8) -> Self {
        let hsv = Hsv {
            h: h % 360,
            s: s.min(100),
            v: v.min(100),
        };
        hsv.to_rgb()
    }

    /// Add alpha channel
    pub const fn with_alpha(self, alpha: u8) -> ColorAlpha {
        ColorAlpha {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    /// Darken color by amount (0-255)
    pub fn darken(self, amount: u8) -> Self {
        Self {
            r: self.r.saturating_sub(amount),
            g: self.g.saturating_sub(amount),
            b: self.b.saturating_sub(amount),
        }
    }

    /// Lighten color by amount (0-255)
    pub fn lighten(self, amount: u8) -> Self {
        Self {
            r: self.r.saturating_add(amount),
            g: self.g.saturating_add(amount),
            b: self.b.saturating_add(amount),
        }
    }

    /// Mix two colors with ratio (0 = all self, 255 = all other)
    pub fn mix(self, other: Color, ratio: u8) -> Self {
        let inv = 255 - ratio;
        Self {
            r: ((self.r as u16 * inv as u16 + other.r as u16 * ratio as u16) / 255) as u8,
            g: ((self.g as u16 * inv as u16 + other.g as u16 * ratio as u16) / 255) as u8,
            b: ((self.b as u16 * inv as u16 + other.b as u16 * ratio as u16) / 255) as u8,
        }
    }

    /// Convert to HSV
    pub fn to_hsv(self) -> Hsv {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let s = if max == 0.0 { 0.0 } else { delta / max };
        let v = max;

        Hsv {
            h: (h + 360.0) as u16 % 360,
            s: (s * 100.0) as u8,
            v: (v * 100.0) as u8,
        }
    }

    /// Convert to grayscale
    pub fn to_grayscale(self) -> Self {
        // Use standard luminance formula
        let gray = ((self.r as u16 * 77 + self.g as u16 * 150 + self.b as u16 * 29) / 256) as u8;
        Self::rgb(gray, gray, gray)
    }

    /// Convert to RGB565 format
    pub fn to_rgb565(self) -> u16 {
        let r = (self.r as u16 >> 3) & 0x1F;
        let g = (self.g as u16 >> 2) & 0x3F;
        let b = (self.b as u16 >> 3) & 0x1F;
        (r << 11) | (g << 5) | b
    }

    /// Convert to 32-bit RGBA (with alpha = 255)
    pub fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | 0xFF
    }

    // Common color constants
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const ORANGE: Color = Color::rgb(255, 165, 0);
    pub const PURPLE: Color = Color::rgb(128, 0, 128);
}

impl ColorAlpha {
    /// Create a new RGBA color
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from Color with specified alpha
    pub const fn from_color(color: Color, alpha: u8) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: alpha,
        }
    }

    /// Create from RGB color with opacity value
    pub const fn from_rgb(color: Color, opacity: Opacity) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: opacity.value(),
        }
    }

    /// Get RGB part (drop alpha)
    pub const fn to_color(self) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }

    /// Convert to 32-bit RGBA
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Create from 32-bit RGBA
    pub const fn from_u32(rgba: u32) -> Self {
        Self {
            r: ((rgba >> 24) & 0xFF) as u8,
            g: ((rgba >> 16) & 0xFF) as u8,
            b: ((rgba >> 8) & 0xFF) as u8,
            a: (rgba & 0xFF) as u8,
        }
    }
}

impl Opacity {
    /// Transparent (0)
    pub const TRANSPARENT: Opacity = Opacity(0);
    pub const OPA_0: Opacity = Opacity(0);
    pub const OPA_10: Opacity = Opacity(25);
    pub const OPA_20: Opacity = Opacity(51);
    pub const OPA_30: Opacity = Opacity(76);
    pub const OPA_40: Opacity = Opacity(102);
    pub const OPA_50: Opacity = Opacity(127);
    pub const OPA_60: Opacity = Opacity(153);
    pub const OPA_70: Opacity = Opacity(178);
    pub const OPA_80: Opacity = Opacity(204);
    pub const OPA_90: Opacity = Opacity(229);
    pub const OPA_100: Opacity = Opacity(255);
    /// Opaque (255)
    pub const OPAQUE: Opacity = Opacity(255);
    pub const COVER: Opacity = Opacity(255);

    /// Create from raw value (0-255)
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Create from percentage (0-100)
    pub fn from_percent(percent: u8) -> Self {
        let clamped = percent.min(100);
        Self((clamped as u16 * 255 / 100) as u8)
    }

    /// Get raw value (0-255)
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Check if fully transparent
    pub const fn is_transparent(self) -> bool {
        self.0 == 0
    }

    /// Check if fully opaque
    pub const fn is_opaque(self) -> bool {
        self.0 == 255
    }
}

impl From<u8> for Opacity {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<f32> for Opacity {
    fn from(value: f32) -> Self {
        Self((value.clamp(0.0, 1.0) * 255.0) as u8)
    }
}

impl Hsv {
    /// Convert HSV to RGB
    pub fn to_rgb(self) -> Color {
        let h = self.h % 360;
        let s = self.s.min(100) as f32 / 100.0;
        let v = self.v.min(100) as f32 / 100.0;

        let c = v * s;
        let x = c * (1.0 - ((h as f32 / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match h {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Color::rgb(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
}

impl ColorFormat {
    /// Get bytes per pixel for this format
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            ColorFormat::L8 | ColorFormat::A8 => 1,
            ColorFormat::Rgb565 => 2,
            ColorFormat::Rgb888 => 3,
            ColorFormat::Argb8888 | ColorFormat::Xrgb8888 => 4,
        }
    }

    /// Get bits per pixel for this format
    pub const fn bits_per_pixel(self) -> usize {
        self.bytes_per_pixel() * 8
    }

    /// Check if format has alpha channel
    pub const fn has_alpha(self) -> bool {
        matches!(self, ColorFormat::A8 | ColorFormat::Argb8888)
    }
}

// Conversion from old Rgba8888 type (for compatibility during migration)
use super::super::color::Rgba8888;

impl From<Rgba8888> for ColorAlpha {
    fn from(rgba: Rgba8888) -> Self {
        let raw = rgba.to_u32();
        Self::from_u32(raw)
    }
}

impl From<ColorAlpha> for Rgba8888 {
    fn from(color: ColorAlpha) -> Self {
        Rgba8888::rgba(color.r, color.g, color.b, color.a)
    }
}

impl From<Color> for Rgba8888 {
    fn from(color: Color) -> Self {
        Rgba8888::rgba(color.r, color.g, color.b, 255)
    }
}
