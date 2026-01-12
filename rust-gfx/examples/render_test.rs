use rust_gfx::{Circle, Rgb565Rasterizer, RoundedRect, Rgba8888, Shape, Arc, Line};
use std::fs::File;
use std::io::BufWriter;

fn main() {
    const WIDTH: usize = 240;
    const HEIGHT: usize = 240;

    let mut rasterizer = Rgb565Rasterizer::new(WIDTH, HEIGHT);

    // Clear to black
    rasterizer.clear();

    // Test 1: Filled circle
    let circle = Circle::new(60, 60, 40)
        .fill_solid(Rgba8888::rgb(255, 0, 0));
    circle.draw(&mut rasterizer);

    // Test 2: Stroked circle with AA
    let circle2 = Circle::new(180, 60, 30)
        .stroke(3, Rgba8888::rgb(0, 255, 0));
    circle2.draw(&mut rasterizer);

    // Test 3: Rounded rectangle with gradient fill
    let rect = RoundedRect::new(40, 120, 80, 60, 10, 10, 10, 10)
        .fill_linear_h(Rgba8888::rgb(0, 0, 255), Rgba8888::rgb(255, 255, 0));
    rect.draw(&mut rasterizer);

    // Test 4: Arc
    let arc = Arc::new(180, 160, 35, 0, 270)
        .stroke(4, Rgba8888::rgb(255, 0, 255));
    arc.draw(&mut rasterizer);

    // Test 5: Line
    let line = Line::new(20, 200, 220, 220)
        .stroke(2, Rgba8888::rgb(255, 255, 255));
    line.draw(&mut rasterizer);

    // Convert RGB565 to RGB8 for PNG
    let rgb565_buffer = rasterizer.buffer();
    let mut rgb8_buffer = vec![0u8; WIDTH * HEIGHT * 3];
    
    for i in 0..WIDTH * HEIGHT {
        let pixel = ((rgb565_buffer[i * 2] as u16) << 8) | (rgb565_buffer[i * 2 + 1] as u16);
        let r = ((pixel >> 11) & 0x1F) as u8;
        let g = ((pixel >> 5) & 0x3F) as u8;
        let b = (pixel & 0x1F) as u8;
        
        // Scale to 8-bit
        rgb8_buffer[i * 3] = (r as u16 * 255 / 31) as u8;
        rgb8_buffer[i * 3 + 1] = (g as u16 * 255 / 63) as u8;
        rgb8_buffer[i * 3 + 2] = (b as u16 * 255 / 31) as u8;
    }

    // Write PNG
    let file = File::create("output.png").unwrap();
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&rgb8_buffer).unwrap();

    println!("✓ Rendered graphics test to output.png");
    println!("  - Red filled circle");
    println!("  - Green stroked circle");
    println!("  - Blue-to-yellow gradient rounded rect");
    println!("  - Magenta arc (0-270°)");
    println!("  - White line");
}
