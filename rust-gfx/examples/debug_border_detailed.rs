/// Debug border to understand opa=0 issue  
use rust_gfx::*;
use rust_gfx::primitives::*;

const SPRITE_WIDTH: usize = 102;
const SPRITE_HEIGHT: usize = 125;

fn main() {
    // Test border_w1_full - the failing case
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

    println!("Drawing rect with:");
    println!("  area: (20,30)-(82,95)");
    println!("  radius: {}", dsc.radius);
    println!("  border_width: {}", dsc.border_width);
    println!("  border_opa: {}", dsc.border_opa);
    println!("  bg_opa: {}", dsc.bg_opa);

    let area = Area::new(20, 30, 82, 95);
    
    // Manually calculate what should happen
    let outer = area;
    let inner = Area::new(21, 31, 81, 94);
    let core = Area::new(30, 40, 72, 85);
    
    println!("\nCalculated areas:");
    println!("  outer: ({},{})-({},{})", outer.x1, outer.y1, outer.x2, outer.y2);
    println!("  inner: ({},{})-({},{})", inner.x1, inner.y1, inner.x2, inner.y2);
    println!("  core: ({},{})-({},{})", core.x1, core.y1, core.x2, core.y2);
    println!("\nTop straight edge should cover:");
    println!("  y: {} to {}", outer.y1, inner.y1.saturating_sub(1));
    println!("  x: {} to {}", core.x1, core.x2);
    println!("  This includes pixels (46,30) and (90,30)");
    
    draw_rect(&mut canvas, &dsc, &area);

    // Check specific failing pixels
    println!("\nPixel values after drawing:");
    println!("  (46,30): {:?}", canvas.get_pixel(46, 30));
    println!("  (90,30): {:?}", canvas.get_pixel(90, 30));
    println!("  (30,30): {:?} (should be yellow, in straight edge)", canvas.get_pixel(30, 30));
    println!("  (72,30): {:?} (should be yellow, in straight edge)", canvas.get_pixel(72, 30));
    
    bmp::save_bmp(&canvas, "debug_border_detailed.bmp").expect("Failed to save");
}
