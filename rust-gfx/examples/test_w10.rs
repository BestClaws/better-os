use rust_gfx::*;
use rust_gfx::primitives::*;

fn main() {
    let mut canvas = Canvas::new(102, 125);
    canvas.clear(Rgba8888::TRANSPARENT);

    let mut dsc = RectDsc::new();
    dsc.radius = 10;
    dsc.bg_opa = OPA_COVER;
    dsc.bg_color = Rgba8888::rgb(50, 50, 50);

    let area = Area::new(20, 30, 82, 95);
    
    // Draw only background
    draw_rect(&mut canvas, &dsc, &area);

    // Check pixels after bg draw
    let pixels = [(26,29), (27,29), (28,29)];
    println!("After bg draw:");
    for &(x, y) in &pixels {
        let color = canvas.get_pixel(x, y);
        println!("  ({},{}) = {:?}", x, y, (color.r(), color.g(), color.b(), color.a()));
    }
}
