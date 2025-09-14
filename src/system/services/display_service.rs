use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::info;

use crate::system::hal::display::{AsyncDisplay, PixelFormat, DisplayResolution, DisplayCapabilities};
use crate::libs::gfx::two_d::Rect;

/// High-level display service that abstracts display driver details
/// and exposes unified parameters for the compositor and apps.
pub struct DisplayService {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    pixel_format: PixelFormat,
    resolution: DisplayResolution,
}

impl DisplayService {
    /// Initialize DisplayService by negotiating pixel format and resolution with the driver.
    pub async fn init(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) -> Self {
        // Query capabilities and choose preferred/defaults from driver
        let (caps, mut chosen_format, mut chosen_resolution) = {
            let mut l = driver.lock().await;
            let caps = l.capabilities();
            let fmt = caps.preferred_format;
            let res = caps.preferred_resolution;
            // Ensure driver is configured to the chosen resolution
            l.set_resolution(res);
            (caps, fmt, res)
        };

        info!(
            "DisplayService negotiated: format={:?} scale={} logical={}x{} physical={}x{}",
            chosen_format,
            chosen_resolution.scale,
            chosen_resolution.logical.width,
            chosen_resolution.logical.height,
            chosen_resolution.physical.width,
            chosen_resolution.physical.height
        );

        Self { driver, pixel_format: chosen_format, resolution: chosen_resolution }
    }

    /// Query capabilities from the underlying driver.
    pub async fn driver_capabilities(&self) -> DisplayCapabilities {
        let mut l = self.driver.lock().await;
        l.capabilities()
    }

    /// Native pixel format used by the display pipeline.
    pub fn pixel_format(&self) -> PixelFormat { self.pixel_format }
    /// Logical framebuffer width in pixels.
    pub fn width(&self) -> u32 { self.resolution.logical.width }
    /// Logical framebuffer height in pixels.
    pub fn height(&self) -> u32 { self.resolution.logical.height }
    /// Active resolution
    pub fn resolution(&self) -> DisplayResolution { self.resolution }

    /// Change active resolution. Driver mode switch should be performed by caller.
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


