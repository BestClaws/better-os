use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::services::display::{Display, DisplayService};
use crate::system::tasks::{benchmark_luma4, benchmark_rgb565, font_demo_luma4, font_demo_rgb565};

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
    
    // RGB565 = 16 bits per pixel (2 bytes per pixel)
    let mut rgb565_buffer = alloc::vec![0u8; pixel_count * 2];
    // Luma4 = 4 bits per pixel (2 pixels per byte)
    let mut luma4_buffer = alloc::vec![0u8; (pixel_count + 1) / 2];

    info!("Starting RGB565 and Luma4 benchmark loop");

    let mut frame_counter = 0u32;
    loop {
        frame_counter += 1;
        //
        // info!("=== RGB565 Benchmarks ===");
        // benchmark_rgb565::run(display, &mut rgb565_buffer, width_u16, height_u16).await;
        
        // info!("=== Luma4 Benchmarks ===");
        // benchmark_luma4::run(display, &mut luma4_buffer, width_u16, height_u16).await;

        info!("=== Luma4 Font Demo ===");
        font_demo_luma4::run_font_demo(&mut luma4_buffer, width_u16, height_u16, frame_counter);

        // Draw to display
        let mut display_lock = display.lock().await;
        display_lock.draw(&luma4_buffer).await;
        drop(display_lock);

        // info!("=== RGB565 Font Demo ===");
        // font_demo_rgb565::run_font_demo(&mut rgb565_buffer, width_u16, height_u16, frame_counter);
        //
        // // Draw to display
        // let mut display_lock = display.lock().await;
        // display_lock.draw(&rgb565_buffer).await;
        // drop(display_lock);
    }
}
