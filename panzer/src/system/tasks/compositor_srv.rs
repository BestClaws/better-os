use alloc::boxed::Box;
use alloc::vec::Vec;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::services::display::{Display, DisplayService};

use gfx::rasterizer::RasterTarget;
use gfx::colors::Color;
use gfx::primitives::font::Font;
use gfx::rgb565::Rgb565Rasterizer;

/// Embedded pixel font
const FONT_DATA_PIXEL: &[u8] = include_bytes!("../../assets/pixel.ttf");

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
    let mut display_lock = display.lock().await;
    display_lock.init().await;
    display_lock.set_brightness(0xFF).await;
    info!("Display initialized: brightness=100%");
    drop(display_lock);

    let width = resolution.logical.width as usize;
    let height = resolution.logical.height as usize;
    let pixel_count = width * height;
    let mut frame_buffer = alloc::vec![0u8; pixel_count * 2]; // RGB565 = 2 bytes per pixel

    // Build font with builder pattern
    let cache_start = Instant::now();
    let font = Font::builder()
        .data(FONT_DATA_PIXEL)
        .size(8.0)
        .cache("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 !?.:,'-Frame\u{00b5}s")
        .hint(true)
        .build();
    let cache_time = cache_start.elapsed().as_millis();
    info!("Font built in {} ms", cache_time);

    // Red background (RGB565 format)
    let bg_color = (0xF8, 0x00); // Pure red: 0xF800
    let mut frame_counter = 0u32;

    info!("Starting font rendering demo");
    
    loop {
        let frame_start = Instant::now();
        let render_start = Instant::now();
        frame_counter += 1;
        
        // Fill with solid red background
        for chunk in frame_buffer.chunks_exact_mut(2) {
            chunk[0] = bg_color.0;
            chunk[1] = bg_color.1;
        }
        
        let render_time = render_start.elapsed().as_micros();
        
        // Create rasterizer for text drawing
        let mut rasterizer = Rgb565Rasterizer::new(&mut frame_buffer, width as u16, height as u16);
        let white = Color::rgba(255, 255, 255, 255);
        
        // Draw text using font
        let text_start = Instant::now();
        
        font.draw_text(&mut rasterizer, "THE QUICK BROWN FOX JUMPS OVER", 10, 15 + font.baseline(), white);
        font.draw_text(&mut rasterizer, "THE LAZY DOG. PACK MY BOX WITH", 10, 30 + font.baseline(), white);
        font.draw_text(&mut rasterizer, "FIVE DOZEN LIQUOR JUGS. HOW", 10, 45 + font.baseline(), white);
        font.draw_text(&mut rasterizer, "VEXINGLY QUICK DAFT ZEBRAS JUMP.", 10, 60 + font.baseline(), white);
        
        // Draw dynamic frame counter
        use core::fmt::Write;
        let mut frame_text_buf = heapless::String::<32>::new();
        let _ = write!(frame_text_buf, "Frame: {}", frame_counter);
        font.draw_text(&mut rasterizer, &frame_text_buf, 10, 80 + font.baseline(), white);
        
        let text_time = text_start.elapsed().as_micros();
        
        info!("Render: {} \u{00b5}s | Text: {} \u{00b5}s", render_time, text_time);
        
        // Draw the frame
        let mut display_lock = display.lock().await;
        display_lock.draw(&frame_buffer).await;
        drop(display_lock);
        
        // Maintain target frame rate
        let frame_time = frame_start.elapsed();
        if frame_time.as_millis() < MIN_FRAME_TIME_MS {
            Timer::after(Duration::from_millis(MIN_FRAME_TIME_MS - frame_time.as_millis())).await;
        }
    }
}
