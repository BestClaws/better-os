use defmt::info;
use embassy_time::{Duration, Instant, Timer};

use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::primitives::{CornerRadius, Rectangle, FillStyle, Gradient, GradientStop, StrokeColor, StrokeStyle};
use gfx::rasterizer::RasterTarget;
use swash::zeno::{Bounds, Point, Stroke};

use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::hal::display::{AsyncDisplay, PixelFormat};

/// Run rectangle benchmark using gfx Rectangle primitive: 80% of frame size, centered
pub async fn run_rect_benchmark(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    buffer: &mut [u8],
    width: u16,
    height: u16,
    frame_counter: u32,
    pixel_format: PixelFormat,
) {
    info!("=== Rectangle Benchmark (Frame {}) ===", frame_counter);

    let (area, clip) = centered_rect_bounds(width, height);

    let mut test_num = 0;

    // Test all combinations: fills x corners x strokes
    for fill_idx in 0..3 {
        for corner_idx in 0..4 {
            let (corner_name, corner_radii) = match corner_idx {
                0 => ("Sharp", [CornerRadius::new(0.0, 0.0); 4]),
                1 => ("R10", [CornerRadius::new(10.0, 10.0); 4]),
                2 => ("R20", [CornerRadius::new(20.0, 20.0); 4]),
                _ => ("MultiR", [
                    CornerRadius::new(0.0, 0.0),
                    CornerRadius::new(10.0, 10.0),
                    CornerRadius::new(20.0, 20.0),
                    CornerRadius::new(30.0, 30.0),
                ]),
            };

            for stroke_idx in 0..6 {
                let (fill_name, fill) = match fill_idx {
                    0 => ("Solid", FillStyle::Solid(Color::rgba(150, 150, 150, 255))),
                    1 => ("VGrad", FillStyle::Gradient(Gradient::Vertical(GradientStop([
                        (Color::rgba(40, 40, 40, 255), 0),
                        (Color::rgba(180, 180, 180, 255), 128),
                        (Color::rgba(255, 255, 255, 255), 255),
                    ])))),
                    _ => ("HGrad", FillStyle::Gradient(Gradient::Horizontal(GradientStop([
                        (Color::rgba(40, 40, 40, 255), 0),
                        (Color::rgba(180, 180, 180, 255), 128),
                        (Color::rgba(255, 255, 255, 255), 255),
                    ])))),
                };

                let (stroke_name, edges) = match stroke_idx {
                    0 => ("NoStroke", [None, None, None, None]),
                    1 => ("1px", [
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                    ]),
                    2 => ("3px", [
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                    ]),
                    3 => ("5px", [
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(5.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(5.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(5.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(5.0) }),
                    ]),
                    4 => ("Asym1357", [
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(5.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(7.0) }),
                    ]),
                    _ => ("AsymCol", [
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 100, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(100, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(100, 100, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::Solid(Color::rgba(255, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                    ]),
                };

                test_num += 1;
                buffer.fill(0);

                let rect = Rectangle {
                    area,
                    fill,
                    edges,
                    clip,
                    corner_radii,
                };

                let t = Instant::now();
                match pixel_format {
                    PixelFormat::Gray4 => {
                        let mut rasterizer = Luma4Rasterizer::new(buffer, width, height);
                        rect.draw(&mut rasterizer);
                    }
                    PixelFormat::Rgb565 => {
                        let mut rasterizer = Rgb565Rasterizer::new(buffer, width, height);
                        rect.draw(&mut rasterizer);
                    }
                }
                info!("{}. {} + {} + {}: {} µs", test_num, fill_name, corner_name, stroke_name, t.elapsed().as_micros());
                present_frame(display, buffer).await;
            }
        }
    }
}

fn centered_rect_bounds(width: u16, height: u16) -> (Bounds, Bounds) {
    let width_f = width as f32;
    let height_f = height as f32;
    let rect_width = width_f * 0.8;
    let rect_height = height_f * 0.8;
    let offset_x = (width_f - rect_width) * 0.5;
    let offset_y = (height_f - rect_height) * 0.5;

    let area = Bounds::new(
        Point::new(offset_x, offset_y),
        Point::new(offset_x + rect_width, offset_y + rect_height),
    );
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width_f, height_f));
    (area, clip)
}

async fn present_frame(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    buffer: &[u8],
) {
    let mut display_lock = display.lock().await;
    display_lock.draw(buffer).await;
    drop(display_lock);
    Timer::after(Duration::from_millis(1000)).await;
}
