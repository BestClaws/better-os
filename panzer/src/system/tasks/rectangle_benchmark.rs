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
    let format_prefix = match pixel_format {
        PixelFormat::Gray4 => "luma4",
        PixelFormat::Rgb565 => "rgb565",
    };

    // Test all combinations: 3 fills x 4 corners x 9 strokes = 108 variants
    for fill_idx in 0..3 {
        for corner_idx in 0..4 {
            let (corner_name, corner_radii) = match corner_idx {
                0 => ("sharp", [CornerRadius::new(0.0, 0.0); 4]),
                1 => ("r4", [CornerRadius::new(4.0, 4.0); 4]),
                2 => ("r6", [CornerRadius::new(6.0, 6.0); 4]),
                _ => ("multi", [
                    CornerRadius::new(0.0, 0.0),
                    CornerRadius::new(4.0, 4.0),
                    CornerRadius::new(6.0, 6.0),
                    CornerRadius::new(8.0, 8.0),
                ]),
            };

            for stroke_idx in 0..9 {
                // Recreate fill each iteration since FillStyle doesn't implement Copy
                let (fill_name, fill) = match fill_idx {
                    0 => ("solid", FillStyle::Solid(Color::rgba(100, 180, 220, 255))),  // Cyan-blue
                    1 => ("vgrad", FillStyle::Gradient(Gradient::Vertical(GradientStop([
                        (Color::rgba(220, 60, 100, 255), 0),     // Pink-red
                        (Color::rgba(120, 180, 240, 255), 128),  // Sky blue
                        (Color::rgba(100, 255, 150, 255), 255),  // Mint green
                    ])))),
                    _ => ("hgrad", FillStyle::Gradient(Gradient::Horizontal(GradientStop([
                        (Color::rgba(240, 180, 60, 255), 0),     // Orange-yellow
                        (Color::rgba(160, 100, 220, 255), 128),  // Purple
                        (Color::rgba(80, 220, 200, 255), 255),   // Turquoise
                    ])))),
                };

                let (stroke_name, edges): (&str, [Option<StrokeStyle<2>>; 4]) = match stroke_idx {
                    0 => ("nostroke", [None, None, None, None]),
                    1 => ("s1", [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                    ]),
                    2 => ("s2", [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                    ]),
                    3 => ("s3", [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                    ]),
                    4 => ("asym_w", [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(1.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(3.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 255, 255)), stroke: Stroke::new(4.0) }),
                    ]),
                    5 => ("asym_c", [
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 100, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(100, 100, 255, 255)), stroke: Stroke::new(2.0) }),
                        Some(StrokeStyle { color: StrokeColor::<2>::Solid(Color::rgba(255, 255, 100, 255)), stroke: Stroke::new(2.0) }),
                    ]),
                    6 => ("grad_h", [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 100, 100, 255), 0),
                                (Color::rgba(100, 100, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                    ]),
                    7 => ("grad_v", [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(100, 255, 100, 255), 0),
                                (Color::rgba(255, 255, 100, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                    ]),
                    _ => ("grad_mix", [
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 80, 80, 255), 0),
                                (Color::rgba(80, 80, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(80, 255, 80, 255), 0),
                                (Color::rgba(255, 255, 80, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                                (Color::rgba(255, 140, 200, 255), 0),
                                (Color::rgba(140, 200, 255, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
                        Some(StrokeStyle { 
                            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                                (Color::rgba(200, 140, 255, 255), 0),
                                (Color::rgba(255, 200, 140, 255), 255),
                            ]))), 
                            stroke: Stroke::new(2.0) 
                        }),
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
                let elapsed = t.elapsed().as_micros();
                
                info!("{}_rect_{:03}_{}_{}_{}  {} µs", format_prefix, test_num, fill_name, corner_name, stroke_name, elapsed);
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
