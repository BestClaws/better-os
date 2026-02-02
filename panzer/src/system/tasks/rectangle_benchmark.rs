use defmt::info;
use embassy_time::Instant;

use crate::system::hal::display::PixelFormat;
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::primitives::{CornerRadius, Rectangle, FillStyle, Gradient, GradientStop, StrokeColor, StrokeStyle};
use gfx::rasterizer::RasterTarget;
use swash::zeno::{Bounds, Point, Stroke};

/// Run rectangle benchmark using gfx Rectangle primitive: 80% of frame size, centered
pub fn run_rect_benchmark(frame_buffer: &mut [u8], width: u16, height: u16, frame_counter: u32, pixel_format: PixelFormat) {
    match pixel_format {
        PixelFormat::Gray4 => run_rect_luma4(frame_buffer, width, height, frame_counter),
        PixelFormat::Rgb565 => run_rect_rgb565(frame_buffer, width, height, frame_counter),
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

fn run_rect_luma4(frame_buffer: &mut [u8], width: u16, height: u16, frame_counter: u32) {
    info!("=== Luma4 Rect Benchmark (Frame {}) ===", frame_counter);

    frame_buffer.fill(0);

    let mut rasterizer = Luma4Rasterizer::new(frame_buffer, width, height);
    let (area, clip) = centered_rect_bounds(width, height);

    let rectangle = Rectangle {
        area,
        fill: FillStyle::Gradient(Gradient::Vertical(GradientStop([
            (Color::rgba(20, 20, 20, 255), 0),
            (Color::rgba(180, 180, 220, 255), 160),
            (Color::rgba(255, 255, 255, 255), 255),
        ]))),
        edges: [
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(255, 160, 120, 255)),
                stroke: Stroke::new(2.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(120, 200, 255, 255)),
                stroke: Stroke::new(2.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(180, 255, 160, 255)),
                stroke: Stroke::new(2.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(255, 220, 180, 255)),
                stroke: Stroke::new(2.0),
            }),
        ],
        clip,
        corner_radii: [
            CornerRadius::new(12.0, 12.0),
            CornerRadius::new(12.0, 12.0),
            CornerRadius::new(12.0, 12.0),
            CornerRadius::new(12.0, 12.0),
        ],
    };

    let render_start = Instant::now();
    rectangle.draw(&mut rasterizer);
    let render_time = render_start.elapsed().as_micros();
    info!("Rect render (80% centered, rounded): {} µs", render_time);
}

fn run_rect_rgb565(frame_buffer: &mut [u8], width: u16, height: u16, frame_counter: u32) {
    info!("=== RGB565 Rect Benchmark (Frame {}) ===", frame_counter);

    frame_buffer.fill(0);

    let mut rasterizer = Rgb565Rasterizer::new(frame_buffer, width, height);
    let (area, clip) = centered_rect_bounds(width, height);

    let rectangle = Rectangle {
        area,
        fill: FillStyle::Gradient(Gradient::Horizontal(GradientStop([
            (Color::rgba(32, 48, 128, 255), 0),
            (Color::rgba(120, 200, 255, 255), 160),
            (Color::rgba(255, 255, 255, 255), 255),
        ]))),
        edges: [
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)),
                stroke: Stroke::new(3.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(255, 200, 120, 255)),
                stroke: Stroke::new(3.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(120, 255, 200, 255)),
                stroke: Stroke::new(3.0),
            }),
            Some(StrokeStyle {
                color: StrokeColor::Solid(Color::rgba(200, 160, 255, 255)),
                stroke: Stroke::new(3.0),
            }),
        ],
        clip,
        corner_radii: [
            CornerRadius::new(18.0, 18.0),
            CornerRadius::new(18.0, 18.0),
            CornerRadius::new(18.0, 18.0),
            CornerRadius::new(18.0, 18.0),
        ],
    };

    let render_start = Instant::now();
    rectangle.draw(&mut rasterizer);
    let render_time = render_start.elapsed().as_micros();
    info!("Rect render (80% centered, rounded): {} µs", render_time);
}
