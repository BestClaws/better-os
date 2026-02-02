use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::services::display::{Display, DisplayService};
use crate::system::tasks::{benchmark, font_demo, rectangle_benchmark};

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
    
    // Allocate buffer based on negotiated pixel format
    let buffer_size = negotiated_pixel_format.framebuffer_size(width as u32, height as u32);
    let mut buffer = alloc::vec![0u8; buffer_size];

    info!("Starting benchmark loop with {:?} format", negotiated_pixel_format);

    let mut frame_counter = 0u32;
    loop {
        frame_counter += 1;

        // Match on pixel format and call unified benchmarks
        match negotiated_pixel_format {
            PixelFormat::Gray4 | PixelFormat::Rgb565 => {
                // Uncomment benchmarks as needed:
                // benchmark::run(display, &mut buffer, width_u16, height_u16, negotiated_pixel_format).await;
                
                // font_demo::run_font_demo(&mut buffer, width_u16, height_u16, frame_counter, negotiated_pixel_format);

                // // Draw to display
                // let mut display_lock = display.lock().await;
                // display_lock.draw(&buffer).await;
                // drop(display_lock);

                rectangle_benchmark::run_rect_benchmark(&mut buffer, width_u16, height_u16, frame_counter, negotiated_pixel_format);
                
                // Draw to display
                let mut display_lock = display.lock().await;
                display_lock.draw(&buffer).await;
                drop(display_lock);
            }
        }
    }
}
