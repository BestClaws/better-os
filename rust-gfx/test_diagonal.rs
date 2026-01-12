// Simple test for diagonal line rendering

use rust_gfx::*;
use rust_gfx::primitives::{LineDsc, draw_line};
use rust_gfx::bmp;

fn main() {
    // Create a simple canvas
    let mut canvas = Canvas::new(200, 200);
    
    // Draw a simple diagonal line
    let line = LineDsc {
        p1: Point::new(50, 50),
        p2: Point::new(150, 150),
        width: 10,  // Thicker line
        color: Rgba8888::rgb(255, 0, 0),
        opa: 255,
        round_start: false,
        round_end: false,
        dash_width: 0,
        dash_gap: 0,
    };
    
    draw_line(&mut canvas, &line);
    
    // Save to BMP
    bmp::save_bmp(&canvas, "test_diagonal.bmp").expect("Failed to save BMP");
    
    println!("Saved test_diagonal.bmp");
    
    // Count non-background pixels
    let mut pixel_count = 0;
    for y in 0..200 {
        for x in 0..200 {
            let pixel = canvas.get_pixel(x, y);
            if pixel.alpha() > 0 {
                pixel_count += 1;
            }
        }
    }
    
    println!("Non-background pixels: {}", pixel_count);
}
