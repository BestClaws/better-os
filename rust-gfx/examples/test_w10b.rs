use rust_gfx::primitives::*;
use rust_gfx::*;

fn main() {
    let mut canvas = Canvas::new(102, 125);
    canvas.clear(Rgba8888::TRANSPARENT);

    let mut dsc = RectDsc::new();
    dsc.radius = 10;
    dsc.bg_opa = OPA_COVER;
    dsc.bg_color = Rgba8888::rgb(50, 50, 50);
    dsc.border_opa = OPA_COVER;
    dsc.border_width = 10;
    dsc.border_color = Rgba8888::rgb(255, 255, 0);
    dsc.border_side = BorderSide::FULL;

    let area = Area::new(20, 30, 82, 95);

    draw_rect(&mut canvas, &dsc, &area);

    // Check pixels
    let pixels = [(26, 29), (27, 29), (28, 29)];
    println!("After full draw:");
    for &(x, y) in &pixels {
        let color = canvas.get_pixel(x, y);
        println!(
            "  ({},{}) = {:?}",
            x,
            y,
            (color.r(), color.g(), color.b(), color.a())
        );
    }

    println!("\nExpected from LVGL:");
    println!("  (26,29) = (255,255,0,80)");
    println!("  (27,29) = (255,255,0,160)");
    println!("  (28,29) = (255,255,0,224)");
}
