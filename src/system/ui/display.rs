use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::hal::display::{
    AsyncDisplay, DisplayCapabilities, DisplayResolution, DisplaySize, PixelFormat,
};
use crate::util::math::primitives::Rect;

/// Preferred formats and logical sizes advertised by the UI display service.
pub struct DisplayPreferences {
    pixel_formats: &'static [PixelFormat],
    logical_resolutions: &'static [DisplaySize],
}

impl DisplayPreferences {
    pub const fn new(
        pixel_formats: &'static [PixelFormat],
        logical_resolutions: &'static [DisplaySize],
    ) -> Self {
        Self {
            pixel_formats,
            logical_resolutions,
        }
    }

    pub const fn default() -> Self {
        const FORMATS: &[PixelFormat] = &[PixelFormat::Gray4, PixelFormat::Rgb565];
        const RESOLUTIONS: &[DisplaySize] =
            &[DisplaySize::new(205, 251), DisplaySize::new(102, 125)];
        Self::new(FORMATS, RESOLUTIONS)
    }

    pub fn pixel_formats(&self) -> &'static [PixelFormat] {
        self.pixel_formats
    }

    pub fn logical_resolutions(&self) -> &'static [DisplaySize] {
        self.logical_resolutions
    }
}

/// Negotiates capabilities with the driver and exposes drawing primitives.
pub struct DisplayService {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    preferences: DisplayPreferences,
}

impl DisplayService {
    pub const fn new(
        driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) -> Self {
        Self {
            driver,
            preferences: DisplayPreferences::default(),
        }
    }

    pub fn preferences(&self) -> &DisplayPreferences {
        &self.preferences
    }

    pub async fn initialize(&self) -> Display {
        let (pixel_format, resolution) = {
            let mut guard = self.driver.lock().await;
            let caps = guard.capabilities();
            let format = negotiate_format(&self.preferences, &caps);
            let resolution = negotiate_resolution(&self.preferences, &caps);
            guard.set_pixel_format(format);
            guard.set_resolution(resolution);
            (format, resolution)
        };

        Display {
            driver: self.driver,
            pixel_format,
            resolution,
        }
    }

    pub async fn driver_capabilities(&self) -> DisplayCapabilities {
        let mut guard = self.driver.lock().await;
        guard.capabilities()
    }
}

/// UI-level Display facade that exposes logical framebuffer properties and draw methods.
pub struct Display {
    driver: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    pixel_format: PixelFormat,
    resolution: DisplayResolution,
}

impl Display {
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn width(&self) -> u32 {
        self.resolution.logical.width
    }

    pub fn height(&self) -> u32 {
        self.resolution.logical.height
    }

    pub fn resolution(&self) -> DisplayResolution {
        self.resolution
    }

    pub fn framebuffer_size(&self, width: u32, height: u32) -> usize {
        self.pixel_format.framebuffer_size(width, height)
    }

    pub async fn draw_full(&self, buffer: &[u8]) {
        let mut guard = self.driver.lock().await;
        guard.draw(buffer).await;
    }

    pub async fn draw_region(&self, buffer: &[u8], region: Rect) {
        let mut guard = self.driver.lock().await;
        guard.draw_region(buffer, region).await;
    }
}

fn negotiate_format(preferences: &DisplayPreferences, caps: &DisplayCapabilities) -> PixelFormat {
    for preference in preferences.pixel_formats() {
        if caps.supported_formats.contains(preference) {
            return *preference;
        }
    }
    caps.preferred_format
}

fn negotiate_resolution(
    preferences: &DisplayPreferences,
    caps: &DisplayCapabilities,
) -> DisplayResolution {
    for logical in preferences.logical_resolutions() {
        if let Some(mode) = caps
            .supported_resolutions
            .iter()
            .copied()
            .find(|candidate| candidate.logical == *logical)
        {
            return mode;
        }
    }
    caps.preferred_resolution
}
