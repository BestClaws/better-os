use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::services::display::{Display, DisplayService};
use crate::system::tasks::benchmark_luma4;

/// Compositor service task - handles UI rendering, animations, and display upkeep.
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!("Starting compositor service");

    // Negotiate display capabilities first (before hardware init)
    let display_facade: Display = DisplayService::new(display).initialize().await;
    let resolution = display_facade.resolution();
    let negotiated_pixel_format = display_facade.pixel_format();

    // Initialize display hardware with negotiated settings
    let mut display_lock = display.lock().await;
    display_lock.init().await;
    display_lock.set_brightness(0xFF).await;
    info!("Display initialized: brightness=100%");
    drop(display_lock);

    let width = resolution.logical.width as usize;
    let height = resolution.logical.height as usize;
    let width_u16 = width as u16;
    let height_u16 = height as u16;
    let pixel_count = width * height;
    let mut frame_buffer = alloc::vec![0u8; (pixel_count + 1) / 2]; // Luma4 = 4 bits per pixel (2 pixels per byte)

    info!("Starting Luma4 benchmark loop");

    loop {
        benchmark_luma4::run(display, &mut frame_buffer, width_u16, height_u16).await;
    }
}
