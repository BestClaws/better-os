use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;

pub async fn run_rasterizer_benchmark(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    buffer: &mut [u8],
    width: u16,
    height: u16,
    pixel_format: PixelFormat,
) {
    if width == 0 || height == 0 {
        info!("Rasterizer benchmark skipped: zero dimension framebuffer");
        return;
    }

    let format_name = match pixel_format {
        PixelFormat::Gray4 => "Luma4",
        PixelFormat::Rgb565 => "RGB565",
    };
    info!("Running {} rasterizer benchmarks", format_name);

    let width_usize = width as usize;
    let height_usize = height as usize;
    let (coverage_row, coverage_col, color_row, color_col) =
        build_gradient_vectors(width_usize, height_usize);

    let solid_color = Color::rgba(200, 200, 200, 255);
    let blend_color = Color::rgba(150, 150, 150, 200);

    // Test 1: fill_solid_hspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.fill_solid_hspan(y as u16, 0, solid_color, width);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.fill_solid_hspan(y as u16, 0, solid_color, width);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("fill_solid_hspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    // Test 2: blend_solid_hspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.blend_solid_hspan(y as u16, 0, blend_color, &coverage_row);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.blend_solid_hspan(y as u16, 0, blend_color, &coverage_row);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("blend_solid_hspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    // Test 3: blend_color_hspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.blend_color_hspan(y as u16, 0, &color_row, &coverage_row);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for y in 0..height_usize {
                rasterizer.blend_color_hspan(y as u16, 0, &color_row, &coverage_row);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("blend_color_hspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    // Test 4: fill_solid_vspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.fill_solid_vspan(x as u16, 0, solid_color, height);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.fill_solid_vspan(x as u16, 0, solid_color, height);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("fill_solid_vspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    // Test 5: blend_solid_vspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.blend_solid_vspan(x as u16, 0, blend_color, &coverage_col);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.blend_solid_vspan(x as u16, 0, blend_color, &coverage_col);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("blend_solid_vspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    // Test 6: blend_color_vspan
    buffer.fill(0);
    let start = Instant::now();
    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.blend_color_vspan(x as u16, 0, &color_col, &coverage_col);
            }
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
            for x in 0..width_usize {
                rasterizer.blend_color_vspan(x as u16, 0, &color_col, &coverage_col);
            }
        }
    }
    let elapsed = start.elapsed();
    info!("blend_color_vspan: {} us", elapsed.as_micros());
    present_frame(display, buffer).await;

    info!("{} rasterizer benchmarks complete", format_name);
}

async fn present_frame(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    buffer: &[u8],
) {
    let mut display_lock = display.lock().await;
    display_lock.draw(buffer).await;
    drop(display_lock);
    Timer::after(Duration::from_millis(1_000)).await;
}

fn build_gradient_vectors(
    width: usize,
    height: usize,
) -> (Vec<u8>, Vec<u8>, Vec<Color>, Vec<Color>) {
    let mut coverage_row: Vec<u8> = vec![255; width];
    let mut coverage_col: Vec<u8> = vec![255; height];
    let mut color_row: Vec<Color> = vec![Color::rgba(0, 0, 0, 0); width];
    let mut color_col: Vec<Color> = vec![Color::rgba(0, 0, 0, 0); height];

    let width_den = width.saturating_sub(1).max(1);
    let height_den = height.saturating_sub(1).max(1);

    for (idx, coverage) in coverage_row.iter_mut().enumerate() {
        *coverage = ((idx * 255) / width_den) as u8;
    }
    for (idx, coverage) in coverage_col.iter_mut().enumerate() {
        *coverage = ((idx * 255) / height_den) as u8;
    }

    for (idx, color) in color_row.iter_mut().enumerate() {
        let t = ((idx * 255) / width_den) as u8;
        *color = Color::rgba(t, 255u8.wrapping_sub(t), t, 200);
    }
    for (idx, color) in color_col.iter_mut().enumerate() {
        let t = ((idx * 255) / height_den) as u8;
        *color = Color::rgba(255u8.wrapping_sub(t), t, t, 160);
    }

    (coverage_row, coverage_col, color_row, color_col)
}
