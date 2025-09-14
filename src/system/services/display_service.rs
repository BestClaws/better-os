use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::info;

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use crate::libs::gfx::two_d::Rect;
use crate::system::kernel::config::resources::{FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, FRAME_SCALE_FACTOR};

/// Display capabilities describing supported and preferred pixel formats.
#[derive(Clone, Copy, Debug)]
pub struct DisplayCapabilities {
    /// Pixel formats supported by the display pipeline (driver + compositor)
    pub supported_formats: &'static [PixelFormat],
    /// Preferred pixel format for optimal performance/quality
    pub preferred_format: PixelFormat,
}

/// High-level display service that abstracts display driver details
/// and exposes unified parameters for the compositor and apps.
pub struct DisplayService {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    pixel_format: PixelFormat,
    scale: u32,
    width: u32,
    height: u32,
}

impl DisplayService {
    /// Create a new DisplayService.
    ///
    /// - `driver`: Concrete async display driver instance
    /// - `boot_format`: Optional override of pixel format (defaults to preferred)
    /// - `scale`: Optional display scale factor (defaults to platform config)
    pub fn new(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
        boot_format: Option<PixelFormat>,
        scale: Option<u32>,
    ) -> Self {
        // For now use compile-time geometry/scale
        let width = FRAME_BUFFER_WIDTH;
        let height = FRAME_BUFFER_HEIGHT;
        let scale = scale.unwrap_or(FRAME_SCALE_FACTOR);

        // NOTE: Extend when additional formats are supported end-to-end
        let caps = DisplayCapabilities {
            supported_formats: &[PixelFormat::Rgb565],
            preferred_format: PixelFormat::Rgb565,
        };

        let chosen = boot_format.unwrap_or(caps.preferred_format);
        info!("DisplayService init: format={:?} scale={} size={}x{}", chosen, scale, width, height);

        Self { driver, pixel_format: chosen, scale, width, height }
    }

    /// Returns the display capabilities.
    pub fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities {
            supported_formats: &[PixelFormat::Rgb565],
            preferred_format: PixelFormat::Rgb565,
        }
    }

    /// Native pixel format used by the display pipeline.
    pub fn pixel_format(&self) -> PixelFormat { self.pixel_format }
    /// Scale factor used when presenting to the physical display.
    pub fn scale(&self) -> u32 { self.scale }
    /// Logical framebuffer width in pixels.
    pub fn width(&self) -> u32 { self.width }
    /// Logical framebuffer height in pixels.
    pub fn height(&self) -> u32 { self.height }

    /// Compute the required framebuffer size in bytes for the given dimensions.
    pub fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * self.pixel_format.bytes_per_pixel()
    }

    /// Draw a full-frame buffer to the display using the configured scale.
    pub async fn draw_full(&self, buffer: &[u8]) {
        let mut l = self.driver.lock().await;
        l.draw(buffer, self.scale).await;
    }

    /// Draw a rectangular region of the buffer to the display using the configured scale.
    pub async fn draw_region(&self, buffer: &[u8], region: Rect) {
        let mut l = self.driver.lock().await;
        l.draw_region(buffer, region, self.scale).await;
    }
}


