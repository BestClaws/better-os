use rust_gfx::masks::RadiusMask;
use rust_gfx::types::Area;

fn main() {
    let radius = 35;
    let rect = Area::new(0, 0, radius * 2, radius * 2);
    let mask = RadiusMask::new(rect, radius, false);

    println!("radius={}", radius);
    let h = rect.height();
    for rel_y in 0..=radius * 2 {
        let cir_y = if rel_y < radius {
            radius - rel_y - 1
        } else {
            rel_y - (h - radius)
        };
        if let Some((x_start, values)) = mask.debug_line_info(rel_y) {
            println!(
                "rel_y={} cir_y={} cir_x_start={} opa={:?}",
                rel_y, cir_y, x_start, values
            );
        } else {
            println!("rel_y={} cir_y={} none", rel_y, cir_y);
        }
    }
}
