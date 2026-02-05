use defmt::info;
use embassy_time::Instant;

use crate::system::hal::display::PixelFormat;
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::font::Font;
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;

/// Embedded pixel font
const FONT_DATA_PIXEL: &[u8] = include_bytes!("../../assets/Endfield.ttf");

/// Run font rendering demonstration
pub fn run_font_demo(
    frame_buffer: &mut [u8],
    width: u16,
    height: u16,
    frame_counter: u32,
    pixel_format: PixelFormat,
) {
    let cache_start = Instant::now();
    let mut font = Font::builder()
        .data(FONT_DATA_PIXEL)
        .size(8.0)
        .cache(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 !?.:,'-Frame\u{00b5}s",
        )
        .hint(true)
        .build();
    let cache_time = cache_start.elapsed().as_millis();

    let format_name = match pixel_format {
        PixelFormat::Gray4 => "Luma4",
        PixelFormat::Rgb565 => "RGB565",
    };
    info!(
        "=== {} Font Rendering Demo (Frame {}) ===",
        format_name, frame_counter
    );
    info!("Font cache built in {} ms", cache_time);

    // Clear background
    let render_start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let bg_color = 0x77;
            for byte in frame_buffer.iter_mut() {
                *byte = bg_color;
            }
        }
        PixelFormat::Rgb565 => {
            let bg_color = 0x0010u16.to_le_bytes();
            for chunk in frame_buffer.chunks_exact_mut(2) {
                chunk[0] = bg_color[0];
                chunk[1] = bg_color[1];
            }
        }
    }
    let render_time = render_start.elapsed().as_micros();

    // Draw text
    let text_start = Instant::now();
    let white = Color::rgba(255, 255, 255, 255);

    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(frame_buffer, width, height);
            font.draw_text(
                &mut rasterizer,
                "THE QUICK BROWN FOX JUMPS OVER",
                10,
                15 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "THE LAZY DOG. PACK MY BOX WITH",
                10,
                30 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "FIVE DOZEN LIQUOR JUGS. HOW",
                10,
                45 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "VEXINGLY QUICK DAFT ZEBRAS JUMP.",
                10,
                60 + font.baseline(),
                white,
            );

            use core::fmt::Write;
            let mut frame_text_buf = heapless::String::<32>::new();
            let _ = write!(frame_text_buf, "Frame: {}", frame_counter);
            font.draw_text(
                &mut rasterizer,
                &frame_text_buf,
                10,
                80 + font.baseline(),
                white,
            );
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(frame_buffer, width, height);
            font.draw_text(
                &mut rasterizer,
                "THE QUICK BROWN FOX JUMPS OVER",
                10,
                15 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "THE LAZY DOG. PACK MY BOX WITH",
                10,
                30 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "FIVE DOZEN LIQUOR JUGS. HOW",
                10,
                45 + font.baseline(),
                white,
            );
            font.draw_text(
                &mut rasterizer,
                "VEXINGLY QUICK DAFT ZEBRAS JUMP.",
                10,
                60 + font.baseline(),
                white,
            );

            use core::fmt::Write;
            let mut frame_text_buf = heapless::String::<32>::new();
            let _ = write!(frame_text_buf, "Frame: {}", frame_counter);
            font.draw_text(
                &mut rasterizer,
                &frame_text_buf,
                10,
                80 + font.baseline(),
                white,
            );
        }
    }

    let text_time = text_start.elapsed().as_micros();
    info!("Background fill: {} µs", render_time);
    info!("Text rendering: {} µs", text_time);
    info!("Total frame time: {} µs", render_time + text_time);
}
