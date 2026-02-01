use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;

use crate::system::services::display::{Display, DisplayService};

/// Target cadence for the compositor loop (~60 FPS).
const MIN_FRAME_TIME_MS: u64 = 16;

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
    {
        let mut display_lock = display.lock().await;
        display_lock.init().await;
        display_lock.set_brightness(0xFF).await;
        info!("Display initialized: brightness=100%");

        // TEST: Paint screen red to verify RGB565 is working
        info!("TEST: Painting screen red for RGB565 verification");
        let test_width = resolution.logical.width;
        let test_height = resolution.logical.height;
        let pixel_count = (test_width * test_height) as usize;
        let mut test_buffer = alloc::vec![0u8; pixel_count * 2]; // RGB565 = 2 bytes per pixel
        
        // Fill with red: RGB565 red = 0xF800 (R=31, G=0, B=0)
        for i in 0..pixel_count {
            test_buffer[i * 2] = 0xF8;     // High byte
            test_buffer[i * 2 + 1] = 0x00; // Low byte
        }
        
        display_lock.draw(&test_buffer).await;
        info!("TEST: Red screen painted, waiting 2 seconds...");
        Timer::after(Duration::from_secs(2)).await;
        
        panic!("TEST COMPLETE: RGB565 verification done");
    }

   
}
