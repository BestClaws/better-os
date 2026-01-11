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
}
