use rust_gfx::*;
use rust_gfx::primitives::*;

fn main() {
    let mut canvas = Canvas::new(120, 120);
    canvas.clear(Rgba8888::rgb(255, 255, 255));
    
    // Draw diagonal line like reference sprite
    let dsc = LineDsc {
        p1: Point::new(20, 30),
        p2: Point::new(82, 95),
        width: 1,
        color: Rgba8888::rgb(0, 255, 255),
        opa: OPA_COVER,
        dash_width: 0,
        dash_gap: 0,
        round_start: false,
        round_end: false,
    };
    
    draw_line(&mut canvas, &dsc);
    
    bmp::save_bmp(&canvas, "test_diag_line.bmp").expect("Failed to save");
    
    // Print some pixels along the diagonal
    println!("Pixels along diagonal:");
    for i in 0..11 {
        let x = 20 + i * 6;
        let y = 30 + i * 6;
        let color = canvas.get_pixel(x, y);
        println!("  ({:3}, {:3}): {:08X}", x, y, color.to_u32());
    }
}

