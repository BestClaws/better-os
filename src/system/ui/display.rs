use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::util::math::primitives::Rect;
use crate::system::hal::display::{AsyncDisplay, PixelFormat, DisplayResolution, DisplayCapabilities};

/// UI-level Display facade that negotiates with the HAL driver
/// and exposes logical framebuffer properties and draw methods.
pub struct Display {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    pixel_format: PixelFormat,
    resolution: DisplayResolution,
}

impl Display {
    pub async fn init(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) -> Self {
        let (caps, chosen_format, chosen_resolution) = {
            let mut l = driver.lock().await;
            let caps = l.capabilities();
            let fmt = caps.preferred_format;
            let res = caps.preferred_resolution;
            l.set_resolution(res);
            (caps, fmt, res)
        };
        let _ = caps; // reserved for future negotiation policy
        Self { driver, pixel_format: chosen_format, resolution: chosen_resolution }
    }

    pub async fn driver_capabilities(&self) -> DisplayCapabilities {
        let mut l = self.driver.lock().await;
        l.capabilities()
    }

    pub fn pixel_format(&self) -> PixelFormat { self.pixel_format }
    pub fn width(&self) -> u32 { self.resolution.logical.width }
    pub fn height(&self) -> u32 { self.resolution.logical.height }
    pub fn resolution(&self) -> DisplayResolution { self.resolution }

    pub fn set_resolution(&mut self, resolution: DisplayResolution) {
        self.resolution = resolution;
    }

    pub fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * self.pixel_format.bytes_per_pixel()
    }

    pub async fn draw_full(&self, buffer: &[u8]) {
        let mut l = self.driver.lock().await;
        l.draw(buffer).await;
    }

    pub async fn draw_region(&self, buffer: &[u8], region: Rect) {
        let mut l = self.driver.lock().await;
        l.draw_region(buffer, region).await;
    }
}


