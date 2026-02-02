use std::fs;
use bmp::{Image, Pixel};
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;
use gfx::rasterizer::RasterTarget;
use gfx::primitives::{CornerRadius, FillStyle, Gradient, GradientStop, Rectangle, StrokeColor, StrokeStyle};
use zeno::{Bounds, Point, Stroke};

fn main() {
    println!("Generating sprite BMPs...");
    
    // Create output directory
    fs::create_dir_all("output").expect("Failed to create output directory");
    
    // Generate Luma4 sprites (grayscale)
    generate_luma4_sprites();
    
    // Generate RGB565 sprites (color)
    generate_rgb565_sprites();
    
    println!("All sprites generated successfully!");
}

fn generate_luma4_sprites() {
    println!("\n=== Generating Luma4 Sprites ===");
    
    let width = 20u16;
    let height = 20u16;
    let pixel_count = (width as usize) * (height as usize);
    
    // Luma4 = 4 bits per pixel (2 pixels per byte)
    let mut buffer = vec![0u8; (pixel_count + 1) / 2];
    
    // 1. Horizontal solid line (16x1 in center)
    {
        buffer.fill(0);
        let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);
        let y = 10u16; // Center vertically
        let x_start = 2u16; // Start 2 pixels from left
        let solid_color = Color::rgba(255, 255, 255, 255); // White
        rasterizer.fill_solid_hspan(y, x_start, solid_color, 16);
        
        save_luma4_as_bmp(&buffer, width, height, "output/luma4_hline_solid.bmp");
        println!("Generated: luma4_hline_solid.bmp");
    }
    
    // 2. Vertical solid line (1x16 in center)
    {
        buffer.fill(0);
        let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);
        let x = 10u16; // Center horizontally
        let y_start = 2u16; // Start 2 pixels from top
        let solid_color = Color::rgba(255, 255, 255, 255); // White
        rasterizer.fill_solid_vspan(x, y_start, solid_color, 16);
        
        save_luma4_as_bmp(&buffer, width, height, "output/luma4_vline_solid.bmp");
        println!("Generated: luma4_vline_solid.bmp");
    }
    
    // 3. Horizontal gradient line (16x1, red to green in grayscale)
    {
        buffer.fill(0);
        let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);
        let y = 10u16;
        let x_start = 2u16;
        
        // Draw gradient pixel by pixel
        for i in 0..16 {
            let t = ((i * 255) / 15) as u8;
            let red = 255u8 - t;
            let green = t;
            let color = Color::rgba(red, green, 0, 255);
            println!("  Pixel {}: t={} R={} G={} B={} -> Color", i, t, red, green, 0);
            rasterizer.fill_solid_hspan(y, x_start + i, color, 1);
        }
        
        // Debug: Print buffer contents for the gradient row
        println!("Buffer contents after gradient:");
        for x in 0..20 {
            let pixel_idx = (y as usize) * 20 + (x as usize);
            let byte_idx = pixel_idx >> 1;
            let is_high = (pixel_idx & 1) == 0;
            let luma4 = if is_high {
                (buffer[byte_idx] >> 4) & 0x0F
            } else {
                buffer[byte_idx] & 0x0F
            };
            print!("{:2} ", luma4);
        }
        println!();
        
        save_luma4_as_bmp(&buffer, width, height, "output/luma4_hline_gradient.bmp");
        println!("Generated: luma4_hline_gradient.bmp");
    }
    
    // 4. Vertical gradient line (1x16, red to green in grayscale)
    {
        buffer.fill(0);
        let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);
        let x = 10u16;
        let y_start = 2u16;
        
        // Draw gradient pixel by pixel
        for i in 0..16 {
            let t = ((i * 255) / 15) as u8;
            let red = 255u8 - t;
            let green = t;
            let color = Color::rgba(red, green, 0, 255);
            rasterizer.fill_solid_vspan(x, y_start + i, color, 1);
        }
        
        save_luma4_as_bmp(&buffer, width, height, "output/luma4_vline_gradient.bmp");
        println!("Generated: luma4_vline_gradient.bmp");
    }

    // 5. Rounded rectangle demo (16x16 centered)
    {
        buffer.fill(0);
        let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width, height);

        let area = Bounds::new(Point::new(2.0, 2.0), Point::new(18.0, 18.0));
        let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width as f32, height as f32));

        let rectangle = Rectangle {
            area,
            fill: FillStyle::Gradient(Gradient::Vertical(GradientStop([
                (Color::rgba(24, 24, 24, 255), 0),
                (Color::rgba(160, 160, 200, 255), 160),
                (Color::rgba(240, 240, 255, 255), 255),
            ]))),
            edges: [
                Some(StrokeStyle {
                    color: StrokeColor::Gradient(Gradient::Horizontal(GradientStop([
                        (Color::rgba(255, 120, 120, 255), 0),
                        (Color::rgba(255, 255, 255, 255), 255),
                    ]))),
                    stroke: Stroke::new(3.0),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Solid(Color::rgba(120, 255, 200, 255)),
                    stroke: Stroke::new(2.0),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Gradient(Gradient::Horizontal(GradientStop([
                        (Color::rgba(80, 120, 255, 255), 0),
                        (Color::rgba(255, 80, 200, 255), 255),
                    ]))),
                    stroke: Stroke::new(4.0),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Solid(Color::rgba(255, 220, 120, 255)),
                    stroke: Stroke::new(1.5),
                }),
            ],
            clip,
            corner_radii: [
                CornerRadius::new(4.0, 6.0),
                CornerRadius::new(8.0, 8.0),
                CornerRadius::new(6.0, 4.0),
                CornerRadius::new(2.0, 10.0),
            ],
        };

        rectangle.draw(&mut rasterizer);
        save_luma4_as_bmp(&buffer, width, height, "output/luma4_rectangle_demo.bmp");
        println!("Generated: luma4_rectangle_demo.bmp");
    }
}

fn generate_rgb565_sprites() {
    println!("\n=== Generating RGB565 Sprites ===");
    
    let width = 20u16;
    let height = 20u16;
    let pixel_count = (width as usize) * (height as usize);
    
    // RGB565 = 16 bits per pixel (2 bytes per pixel)
    let mut buffer = vec![0u8; pixel_count * 2];
    
    // 1. Horizontal solid line (16x1 in center)
    {
        buffer.fill(0);
        let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);
        let y = 10u16; // Center vertically
        let x_start = 2u16; // Start 2 pixels from left
        let solid_color = Color::rgba(255, 255, 255, 255); // White
        rasterizer.fill_solid_hspan(y, x_start, solid_color, 16);
        
        save_rgb565_as_bmp(&buffer, width, height, "output/rgb565_hline_solid.bmp");
        println!("Generated: rgb565_hline_solid.bmp");
    }
    
    // 2. Vertical solid line (1x16 in center)
    {
        buffer.fill(0);
        let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);
        let x = 10u16; // Center horizontally
        let y_start = 2u16; // Start 2 pixels from top
        let solid_color = Color::rgba(255, 255, 255, 255); // White
        rasterizer.fill_solid_vspan(x, y_start, solid_color, 16);
        
        save_rgb565_as_bmp(&buffer, width, height, "output/rgb565_vline_solid.bmp");
        println!("Generated: rgb565_vline_solid.bmp");
    }
    
    // 3. Horizontal gradient line (16x1, red to green)
    {
        buffer.fill(0);
        let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);
        let y = 10u16;
        let x_start = 2u16;
        
        // Draw gradient pixel by pixel
        for i in 0..16 {
            let t = ((i * 255) / 15) as u8;
            let red = 255u8 - t;
            let green = t;
            let color = Color::rgba(red, green, 0, 255);
            println!("  Pixel {}: t={} R={} G={} B={}", i, t, red, green, 0);
            rasterizer.fill_solid_hspan(y, x_start + i, color, 1);
        }
        
        // Debug: Print buffer contents for the gradient row
        println!("Buffer contents after gradient (RGB565 values):");
        for x in 0..20 {
            let pixel_offset = (y as usize) * 20 + (x as usize);
            let byte_offset = pixel_offset * 2;
            let rgb565 = u16::from_le_bytes([buffer[byte_offset], buffer[byte_offset + 1]]);
            print!("{:04x} ", rgb565);
        }
        println!();
        
        save_rgb565_as_bmp(&buffer, width, height, "output/rgb565_hline_gradient.bmp");
        println!("Generated: rgb565_hline_gradient.bmp");
    }
    
    // 4. Vertical gradient line (1x16, red to green)
    {
        buffer.fill(0);
        let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);
        let x = 10u16;
        let y_start = 2u16;
        
        // Draw gradient pixel by pixel
        for i in 0..16 {
            let t = ((i * 255) / 15) as u8;
            let red = 255u8 - t;
            let green = t;
            let color = Color::rgba(red, green, 0, 255);
            rasterizer.fill_solid_vspan(x, y_start + i, color, 1);
        }
        
        save_rgb565_as_bmp(&buffer, width, height, "output/rgb565_vline_gradient.bmp");
        println!("Generated: rgb565_vline_gradient.bmp");
    }

    // 5. Rounded rectangle demo (16x16 centered)
    {
        buffer.fill(0);
        let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width, height);

        let area = Bounds::new(Point::new(2.0, 2.0), Point::new(18.0, 18.0));
        let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width as f32, height as f32));

        let rectangle = Rectangle {
            area,
            fill: FillStyle::Gradient(Gradient::Horizontal(GradientStop([
                (Color::rgba(40, 180, 220, 255), 0),
                (Color::rgba(120, 80, 220, 255), 128),
                (Color::rgba(255, 120, 200, 255), 255),
            ]))),
            edges: [
                Some(StrokeStyle {
                    color: StrokeColor::Solid(Color::rgba(255, 255, 255, 255)),
                    stroke: Stroke::new(2.0),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Gradient(Gradient::Vertical(GradientStop([
                        (Color::rgba(255, 180, 80, 255), 0),
                        (Color::rgba(80, 255, 200, 255), 255),
                    ]))),
                    stroke: Stroke::new(3.0),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Solid(Color::rgba(60, 120, 255, 255)),
                    stroke: Stroke::new(2.5),
                }),
                Some(StrokeStyle {
                    color: StrokeColor::Gradient(Gradient::Vertical(GradientStop([
                        (Color::rgba(200, 60, 255, 255), 0),
                        (Color::rgba(60, 255, 120, 255), 255),
                    ]))),
                    stroke: Stroke::new(3.5),
                }),
            ],
            clip,
            corner_radii: [
                CornerRadius::new(5.0, 7.0),
                CornerRadius::new(9.0, 9.0),
                CornerRadius::new(5.0, 3.0),
                CornerRadius::new(3.0, 8.0),
            ],
        };

        rectangle.draw(&mut rasterizer);
        save_rgb565_as_bmp(&buffer, width, height, "output/rgb565_rectangle_demo.bmp");
        println!("Generated: rgb565_rectangle_demo.bmp");
    }
}

fn save_luma4_as_bmp(buffer: &[u8], width: u16, height: u16, filename: &str) {
    let mut img = Image::new(width as u32, height as u32);
    
    for y in 0..height {
        for x in 0..width {
            let pixel_idx = (y as usize) * (width as usize) + (x as usize);
            let byte_idx = pixel_idx >> 1;
            let is_high_nibble = (pixel_idx & 1) == 0;
            
            let luma4 = if is_high_nibble {
                (buffer[byte_idx] >> 4) & 0x0F
            } else {
                buffer[byte_idx] & 0x0F
            };
            
            // Expand 4-bit to 8-bit (0-15 to 0-255)
            let luma8 = (luma4 << 4) | luma4;
            
            img.set_pixel(x as u32, y as u32, Pixel::new(luma8, luma8, luma8));
        }
    }
    
    img.save(filename).expect("Failed to save BMP");
}

fn save_rgb565_as_bmp(buffer: &[u8], width: u16, height: u16, filename: &str) {
    let mut img = Image::new(width as u32, height as u32);
    
    for y in 0..height {
        for x in 0..width {
            let pixel_offset = (y as usize) * (width as usize) + (x as usize);
            let byte_offset = pixel_offset * 2;
            
            // Read RGB565 (little-endian)
            let rgb565 = u16::from_le_bytes([buffer[byte_offset], buffer[byte_offset + 1]]);
            
            // Extract RGB565 components
            let r5 = ((rgb565 >> 11) & 0x1F) as u8;
            let g6 = ((rgb565 >> 5) & 0x3F) as u8;
            let b5 = (rgb565 & 0x1F) as u8;
            
            // Expand to 8-bit: replicate high bits to low bits
            let r = (r5 << 3) | (r5 >> 2);
            let g = (g6 << 2) | (g6 >> 4);
            let b = (b5 << 3) | (b5 >> 2);
            
            img.set_pixel(x as u32, y as u32, Pixel::new(r, g, b));
        }
    }
    
    img.save(filename).expect("Failed to save BMP");
}
