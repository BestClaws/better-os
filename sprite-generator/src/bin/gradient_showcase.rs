use std::fs;

use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::{CornerRadius, FillStyle, Gradient, GradientStop, Rectangle, StrokeColor, StrokeStyle};
use gfx::rgb565::Rgb565Rasterizer;
use sprite_generator::{save_luma4_as_bmp, save_rgb565_as_bmp};
use zeno::{Bounds, Point, Stroke};

fn main() {
    let output_dir = "output/demo";
    fs::create_dir_all(output_dir).expect("Failed to create demo output directory");

    let sprite_width = 100u16;
    let sprite_height = 100u16;

    let rect_bounds = Bounds::new(Point::new(10.0, 10.0), Point::new(90.0, 90.0));
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(sprite_width as f32, sprite_height as f32));

    let fill = FillStyle::Gradient(Gradient::Horizontal(GradientStop([
        (Color::rgba(30, 80, 200, 255), 0),
        (Color::rgba(180, 60, 200, 255), 128),
        (Color::rgba(255, 210, 90, 255), 255),
    ])));

    let corner_radii = [
        CornerRadius::new(18.0, 10.0),
        CornerRadius::new(24.0, 14.0),
        CornerRadius::new(12.0, 28.0),
        CornerRadius::new(16.0, 8.0),
    ];

    let edges: [Option<StrokeStyle<'_, 2>>; 4] = [
        Some(StrokeStyle {
            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                (Color::rgba(255, 70, 110, 255), 0),
                (Color::rgba(255, 200, 150, 255), 255),
            ]))),
            stroke: Stroke::new(1.5),
        }),
        Some(StrokeStyle {
            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                (Color::rgba(80, 220, 160, 255), 0),
                (Color::rgba(160, 255, 220, 255), 255),
            ]))),
            stroke: Stroke::new(3.0),
        }),
        Some(StrokeStyle {
            color: StrokeColor::<2>::Gradient(Gradient::Horizontal(GradientStop([
                (Color::rgba(120, 140, 255, 255), 0),
                (Color::rgba(40, 40, 180, 255), 255),
            ]))),
            stroke: Stroke::new(4.5),
        }),
        Some(StrokeStyle {
            color: StrokeColor::<2>::Gradient(Gradient::Vertical(GradientStop([
                (Color::rgba(255, 120, 220, 255), 0),
                (Color::rgba(255, 255, 170, 255), 255),
            ]))),
            stroke: Stroke::new(2.0),
        }),
    ];

    let rectangle = Rectangle {
        area: rect_bounds,
        fill,
        edges,
        clip,
        corner_radii,
    };

    let pixel_count = (sprite_width as usize) * (sprite_height as usize);

    let mut luma4_buffer = vec![0u8; (pixel_count + 1) / 2];
    let mut luma4_rasterizer = Luma4Rasterizer::new(&mut luma4_buffer, sprite_width, sprite_height);
    rectangle.draw(&mut luma4_rasterizer);
    let luma4_path = format!("{}/gradient_rect_luma4.bmp", output_dir);
    save_luma4_as_bmp(&luma4_buffer, sprite_width, sprite_height, &luma4_path);

    let mut rgb565_buffer = vec![0u8; pixel_count * 2];
    let mut rgb565_rasterizer = Rgb565Rasterizer::new(&mut rgb565_buffer, sprite_width, sprite_height);
    rectangle.draw(&mut rgb565_rasterizer);
    let rgb565_path = format!("{}/gradient_rect_rgb565.bmp", output_dir);
    save_rgb565_as_bmp(&rgb565_buffer, sprite_width, sprite_height, &rgb565_path);

    println!("Gradient demo sprite saved to:\n  {}\n  {}", luma4_path, rgb565_path);
}
