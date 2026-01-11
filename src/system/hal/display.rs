use crate::util::math::primitives::Rect;
use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::Format;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub enum PixelFormat {
    Rgb565,
    Rgb888,
    Rgb666,
    Gray8,
    Gray4,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb565 => 2,
            PixelFormat::Rgb888 => 3,
            PixelFormat::Rgb666 => 3,
            PixelFormat::Gray8 => 1,
            PixelFormat::Gray4 => 1, // Note: actual usage needs (width*height+1)/2
        }
    }

    /// Calculate actual framebuffer size in bytes for given dimensions
    pub const fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        let pixels = (width as usize) * (height as usize);
        match self {
            PixelFormat::Gray4 => (pixels + 1) / 2, // 2 pixels per byte, round up
            _ => pixels * self.bytes_per_pixel(),
        }
    }
}

/// Logical or physical size of a display surface in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub struct DisplaySize {
    pub width: u32,
    pub height: u32,
}

impl DisplaySize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Resolution configuration describing logical size rendered by the system,
/// the physical panel size, and the integer scale between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub struct DisplayResolution {
    pub logical: DisplaySize,
    pub physical: DisplaySize,
    /// Integer scale factor applied when presenting logical pixels to the panel.
    /// Example: logical 116x116 on physical 466x466 uses scale 4 (with 2px margin).
    pub scale: u32,
}

/// Display capabilities exposed by a driver.
#[derive(Clone, Copy, Debug, Format)]
pub struct DisplayCapabilities {
    /// Pixel formats supported by the display pipeline (driver + panel)
    pub supported_formats: &'static [PixelFormat],
    /// Preferred pixel format for optimal performance/quality
    pub preferred_format: PixelFormat,
    /// Supported logical resolutions and their mapping to the physical panel
    pub supported_resolutions: &'static [DisplayResolution],
    /// Preferred resolution for boot
    pub preferred_resolution: DisplayResolution,
}

#[async_trait(?Send)]
pub trait AsyncDisplay {
    /// Initialize the display
    async fn init(&mut self);

    /// Paint the entire screen with a single color
    async fn paint_screen(&mut self, color: u8);

    /// Set display brightness
    async fn set_brightness(&mut self, value: u8);

    /// Draw a full logical framebuffer to the display at the current resolution.
    async fn draw(&mut self, buffer: &[u8]);
    /// Draw a logical region of the framebuffer to the display at the current resolution.
    async fn draw_region(&mut self, buffer: &[u8], region: Rect);
    async fn set_orientation(&mut self, orientation: Orientation);
    /// Logical width (in pixels) of the active resolution.
    fn get_width(&self) -> u32;
    /// Logical height (in pixels) of the active resolution.
    fn get_height(&self) -> u32;

    /// Report native pixel format of the driver output buffer
    fn native_pixel_format(&self) -> PixelFormat {
        PixelFormat::Rgb565
    }

    /// Set the active pixel format for the driver
    fn set_pixel_format(&mut self, format: PixelFormat);

    /// Report static capabilities of the display driver/panel.
    fn capabilities(&self) -> DisplayCapabilities;

    /// Set the active resolution. Implementations should validate the requested
    /// resolution against `capabilities()` and choose the closest supported mode
    /// if an exact match is not available.
    fn set_resolution(&mut self, resolution: DisplayResolution);
}

#[derive(Clone, Copy, Format, PartialEq)]
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}
