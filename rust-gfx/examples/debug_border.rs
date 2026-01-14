use rust_gfx::primitives::*;
/// Debug border rendering to understand pixel differences
use rust_gfx::*;

const SPRITE_WIDTH: usize = 102;
const SPRITE_HEIGHT: usize = 125;

fn main() {
    // Test border_w1_full
    let mut canvas = Canvas::new(SPRITE_WIDTH, SPRITE_HEIGHT);
    canvas.clear(Rgba8888::TRANSPARENT);

    let mut dsc = RectDsc::new();
    dsc.radius = 10;
    dsc.bg_opa = OPA_COVER;
    dsc.bg_color = Rgba8888::rgb(50, 50, 50);
    dsc.border_opa = OPA_COVER;
    dsc.border_width = 1;
    dsc.border_color = Rgba8888::rgb(255, 255, 0);
    dsc.border_side = BorderSide::FULL;

    let area = Area::new(20, 30, 82, 95);
    draw_rect(&mut canvas, &dsc, &area);

    // Inspect mask values for top border rows
    let inner_area = Area::new(21, 31, 81, 94);
    let inner_mask = RadiusMask::new(inner_area, 9, true);
    let outer_mask = RadiusMask::new(area, 10, false);
    let mut mask_buf = vec![255u8; (area.x2 - area.x1 + 1) as usize];
    inner_mask.apply_to_line(30, area.x1, &mut mask_buf);
    outer_mask.apply_to_line(30, area.x1, &mut mask_buf);
    println!("Mask row y=30: {:?}", &mask_buf[0..25]);
    mask_buf.fill(255);
    inner_mask.apply_to_line(31, area.x1, &mut mask_buf);
    outer_mask.apply_to_line(31, area.x1, &mut mask_buf);
    println!("Mask row y=31: {:?}", &mask_buf[0..25]);

    // Print specific pixels that should be border
    println!("Pixel (26,30): {:?}", canvas.get_pixel(26, 30));
    println!("Pixel (20,30): {:?}", canvas.get_pixel(20, 30)); // Left edge
    println!("Pixel (82,30): {:?}", canvas.get_pixel(82, 30)); // Right edge
    println!("Pixel (30,30): {:?}", canvas.get_pixel(30, 30)); // Inside border, should be yellow
    println!("Pixel (29,30): {:?}", canvas.get_pixel(29, 30));
    println!("Pixel (31,31): {:?}", canvas.get_pixel(31, 31)); // Should be bg (gray)

    // Check corner pixels
    println!("\nCorner pixels:");
    println!("Pixel (20,40): {:?}", canvas.get_pixel(20, 40)); // Top-left corner region
    println!("Pixel (21,40): {:?}", canvas.get_pixel(21, 40));
    println!("Pixel (22,40): {:?}", canvas.get_pixel(22, 40));

    bmp::save_bmp(&canvas, "debug_border_w1_full.bmp").expect("Failed to save");
    println!("\nSaved debug_border_w1_full.bmp");
}
