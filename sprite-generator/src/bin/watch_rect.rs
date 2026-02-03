use std::fs;

use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::{CornerRadius, Edge, FillStyle, Rectangle, StrokeStyle};
use gfx::rgb565::Rgb565Rasterizer;
use sprite_generator::{save_luma4_as_bmp, save_rgb565_as_bmp};
use zeno::{Bounds, Point, Stroke};

fn main() {
    let output_dir = "output/watch_rect";
    fs::create_dir_all(output_dir).expect("Failed to create watch_rect output directory");

    let width = 120u16;
    let height = 120u16;
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width as f32, height as f32));

    let rectangle = Rectangle::new()
        .bounds(Bounds::new(
            Point::new(20.0, 20.0),
            Point::new((width - 20) as f32, (height - 20) as f32),
        ))
        .corner_radii(CornerRadius::new(24.0, 24.0))
        .fill(FillStyle::solid(Color::rgba(255, 0, 255, 255)))
        .clip(clip)
        .edge(
            Edge::Left,
            StrokeStyle::from_stroke(Stroke::new(12.0)).solid(Color::rgba(255, 255, 0, 255)),
        )
        .build();

    let pixel_count = (width as usize) * (height as usize);

    let mut luma4_buffer = vec![0u8; (pixel_count + 1) / 2];
    let mut luma4_rasterizer = Luma4Rasterizer::new(&mut luma4_buffer, width, height);
    rectangle.draw(&mut luma4_rasterizer);
    let luma4_path = format!("{}/watch_rect_luma4.bmp", output_dir);
    save_luma4_as_bmp(&luma4_buffer, width, height, &luma4_path);

    let mut rgb565_buffer = vec![0u8; pixel_count * 2];
    let mut rgb565_rasterizer = Rgb565Rasterizer::new(&mut rgb565_buffer, width, height);
    rectangle.draw(&mut rgb565_rasterizer);
    let rgb565_path = format!("{}/watch_rect_rgb565.bmp", output_dir);
    save_rgb565_as_bmp(&rgb565_buffer, width, height, &rgb565_path);

    println!("Saved rectangle sprite BMPs:\n  {}\n  {}", luma4_path, rgb565_path);
}
