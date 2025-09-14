use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::info;

use crate::system::hal::display::{AsyncDisplay, PixelFormat, DisplayResolution, DisplayCapabilities, DisplaySize};
use crate::libs::gfx::two_d::Rect;
use crate::system::kernel::config::resources::{FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, FRAME_SCALE_FACTOR};

// Default static capabilities used until drivers expose their own.
static DEFAULT_SUPPORTED_FORMATS: &[PixelFormat] = &[PixelFormat::Rgb565];
static DEFAULT_SUPPORTED_RESOLUTIONS: &[DisplayResolution] = &[
    DisplayResolution {
        logical: DisplaySize { width: FRAME_BUFFER_WIDTH, height: FRAME_BUFFER_HEIGHT },
        physical: DisplaySize { width: FRAME_BUFFER_WIDTH * FRAME_SCALE_FACTOR, height: FRAME_BUFFER_HEIGHT * FRAME_SCALE_FACTOR },
        scale: FRAME_SCALE_FACTOR,
    }
];

/// High-level display service that abstracts display driver details
/// and exposes unified parameters for the compositor and apps.
pub struct DisplayService {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    pixel_format: PixelFormat,
    resolution: DisplayResolution,
}

impl DisplayService {
    /// Create a new DisplayService.
    ///
    /// - `driver`: Concrete async display driver instance
    /// - `boot_format`: Optional override of pixel format (defaults to preferred)
    /// - `override_resolution`: Optional resolution override at boot
    pub fn new(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
        boot_format: Option<PixelFormat>,
        override_resolution: Option<DisplayResolution>,
    ) -> Self {
        // Temporary default based on compile-time geometry while callers migrate.
        let default_resolution = DisplayResolution {
            logical: DisplaySize::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT),
            physical: DisplaySize::new(FRAME_BUFFER_WIDTH * FRAME_SCALE_FACTOR, FRAME_BUFFER_HEIGHT * FRAME_SCALE_FACTOR),
            scale: FRAME_SCALE_FACTOR,
        };

        // NOTE: Extend when additional formats are supported end-to-end
        let chosen_format = boot_format.unwrap_or(PixelFormat::Rgb565);
        let resolution = override_resolution.unwrap_or(default_resolution);

        info!("DisplayService init: format={:?} scale={} logical={}x{} physical={}x{}",
              chosen_format,
              resolution.scale,
              resolution.logical.width, resolution.logical.height,
              resolution.physical.width, resolution.physical.height);

        Self { driver, pixel_format: chosen_format, resolution }
    }

    /// Returns the display capabilities (conservative defaults until drivers expose real caps).
    pub fn capabilities(&self) -> DisplayCapabilities {
        DisplayCapabilities {
            supported_formats: DEFAULT_SUPPORTED_FORMATS,
            preferred_format: PixelFormat::Rgb565,
            supported_resolutions: DEFAULT_SUPPORTED_RESOLUTIONS,
            preferred_resolution: DEFAULT_SUPPORTED_RESOLUTIONS[0],
        }
    }

    /// Native pixel format used by the display pipeline.
    pub fn pixel_format(&self) -> PixelFormat { self.pixel_format }
    /// Logical framebuffer width in pixels.
    pub fn width(&self) -> u32 { self.resolution.logical.width }
    /// Logical framebuffer height in pixels.
    pub fn height(&self) -> u32 { self.resolution.logical.height }
    /// Active resolution
    pub fn resolution(&self) -> DisplayResolution { self.resolution }

    /// Change active resolution. This will request the driver to switch modes.
    pub fn set_resolution(&mut self, resolution: DisplayResolution) {
        self.resolution = resolution;
    }

    /// Compute the required framebuffer size in bytes for the given dimensions.
    pub fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * self.pixel_format.bytes_per_pixel()
    }

    /// Draw a full-frame buffer to the display using the active resolution.
    pub async fn draw_full(&self, buffer: &[u8]) {
        let mut l = self.driver.lock().await;
        l.draw(buffer).await;
    }

    /// Draw a rectangular region of the buffer to the display using the active resolution.
    pub async fn draw_region(&self, buffer: &[u8], region: Rect) {
        let mut l = self.driver.lock().await;
        l.draw_region(buffer, region).await;
    }
}


