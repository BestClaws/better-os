use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use gfx::colors::Color;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::rasterizer::RasterTarget;

#[inline(always)]
fn run_case<F>(buffer: &mut [u8], width: u16, height: u16, mut f: F) -> Duration
where
    F: FnMut(&mut Rgb565Rasterizer<'_>),
{
    buffer.fill(0);
    let start = Instant::now();
    {
        let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
        f(&mut rasterizer);
    }
    start.elapsed()
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

pub async fn run(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    buffer: &mut [u8],
    width: u16,
    height: u16,
) {
    if width == 0 || height == 0 {
        info!("RGB565 benchmark skipped: zero dimension framebuffer");
        return;
    }

    info!("Running RGB565 rasterizer benchmarks");

    let width_usize = width as usize;
    let height_usize = height as usize;

    let mut coverage_row: Vec<u8> = vec![255; width_usize];
    let mut coverage_col: Vec<u8> = vec![255; height_usize];
    let mut color_row: Vec<Color> = vec![Color::rgba(0, 0, 0, 0); width_usize];
    let mut color_col: Vec<Color> = vec![Color::rgba(0, 0, 0, 0); height_usize];

    let width_den = width_usize.saturating_sub(1).max(1);
    let height_den = height_usize.saturating_sub(1).max(1);

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

    let solid_color = Color::rgba(200, 200, 200, 255);
    let blend_color = Color::rgba(150, 150, 150, 200);

    let fill_hspan = run_case(buffer, width, height, |rasterizer| {
        for y in 0..height_usize {
            rasterizer.fill_solid_hspan(y as u16, 0, solid_color, width);
        }
    });
    info!("fill_solid_hspan: {} us", fill_hspan.as_micros());
    present_frame(display, buffer).await;

    let blend_hspan = run_case(buffer, width, height, |rasterizer| {
        for y in 0..height_usize {
            rasterizer.blend_solid_hspan(y as u16, 0, blend_color, &coverage_row);
        }
    });
    info!("blend_solid_hspan: {} us", blend_hspan.as_micros());
    present_frame(display, buffer).await;

    let blend_color_hspan = run_case(buffer, width, height, |rasterizer| {
        for y in 0..height_usize {
            rasterizer.blend_color_hspan(y as u16, 0, &color_row, &coverage_row);
        }
    });
    info!("blend_color_hspan: {} us", blend_color_hspan.as_micros());
    present_frame(display, buffer).await;

    let fill_vspan = run_case(buffer, width, height, |rasterizer| {
        for x in 0..width_usize {
            rasterizer.fill_solid_vspan(x as u16, 0, solid_color, height);
        }
    });
    info!("fill_solid_vspan: {} us", fill_vspan.as_micros());
    present_frame(display, buffer).await;

    let blend_vspan = run_case(buffer, width, height, |rasterizer| {
        for x in 0..width_usize {
            rasterizer.blend_solid_vspan(x as u16, 0, blend_color, &coverage_col);
        }
    });
    info!("blend_solid_vspan: {} us", blend_vspan.as_micros());
    present_frame(display, buffer).await;

    let blend_color_vspan = run_case(buffer, width, height, |rasterizer| {
        for x in 0..width_usize {
            rasterizer.blend_color_vspan(x as u16, 0, &color_col, &coverage_col);
        }
    });
    info!("blend_color_vspan: {} us", blend_color_vspan.as_micros());
    present_frame(display, buffer).await;

    info!("RGB565 rasterizer benchmarks complete");
}
