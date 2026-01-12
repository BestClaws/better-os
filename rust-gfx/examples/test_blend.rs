use rust_gfx::color::*;

fn main() {
    let bg = Rgba8888::rgb(50, 50, 50);  // gray, opaque
    let fg = Rgba8888::rgb(255, 255, 0);  // yellow
    let opa = 160;

    let result = blend_colors(bg, fg, opa);
    println!("blend_colors(bg=(50,50,50,255), fg=(255,255,0), opa=160)");
    println!("  result = ({},{},{},{})", result.r(), result.g(), result.b(), result.a());
    println!("  expected ≈ (176,176,31,255)");
    
    // Manual calculation:
    // r = (255*160 + 50*95) / 255 = (40800 + 4750) / 255 = 45550 / 255 ≈ 178
    let r = (255u32 * 160 + 50 * 95) / 255;
    println!("  manual r = {}", r);
}
