use rust_gfx::masks::RadiusMask;
use rust_gfx::types::Area;

fn dump_radius(label: &str, mask: &RadiusMask, rel_y_start: i32, rel_y_end: i32) {
    println!("{}:", label);
    let mut uniques = std::collections::BTreeSet::new();
    for rel_y in rel_y_start..=rel_y_end {
        #[cfg(debug_assertions)]
        {
            if let Some((x_start, values)) = mask.debug_line_info(rel_y) {
                uniques.extend(values.iter().copied());
                println!("  rel_y={rel_y} x_start={x_start} values={values:?}");
            } else {
                println!("  rel_y={rel_y} <none>");
            }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = (rel_y, mask);
            println!("  rel_y={rel_y} debug info unavailable (release build)");
        }
    }
    #[cfg(debug_assertions)]
    println!("  unique values: {uniques:?}");
}

fn main() {
    let center_x = 51;
    let center_y = 62;
    let radius = 35;
    let width = std::env::args()
        .nth(1)
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(3);

    let outer_area = Area::new(
        center_x - radius,
        center_y - radius,
        center_x + radius,
        center_y + radius,
    );
    let outer_mask = RadiusMask::new(outer_area, radius, false);

    let inner_radius = (radius - width).max(0);
    let inner_area = Area::new(
        center_x - inner_radius,
        center_y - inner_radius,
        center_x + inner_radius,
        center_y + inner_radius,
    );
    let inner_mask = RadiusMask::new(inner_area, inner_radius, true);

    dump_radius("outer", &outer_mask, 0, radius * 2);
    let inner_height = inner_area.height();
    dump_radius("inner", &inner_mask, 0, inner_height);

    let draw_width = outer_area.width() as usize;
    let target_y = 62;
    let mut buf = vec![255u8; draw_width];
    let res = inner_mask.apply(&mut buf, outer_area.x1, target_y);
    println!("\ninner_mask.apply at y={target_y} -> res={res:?}");
    #[cfg(debug_assertions)]
    if let Some((x_start, values)) = inner_mask.debug_line_info(target_y - inner_area.y1) {
        let len_i32 = draw_width as i32;
        let k = inner_area.x1 - outer_area.x1;
        let cir_x_right = k + inner_area.width() - inner_radius + x_start;
        let cir_x_left = k + inner_radius - x_start - 1;
        println!(
            "intermediate: k={k} cir_x_right={cir_x_right} cir_x_left={cir_x_left} x_start={x_start} values={values:?}"
        );
        let clr_start = (cir_x_left + 1).clamp(0, len_i32);
        let clr_len = (cir_x_right - clr_start).clamp(0, len_i32 - clr_start);
        println!("clr_start={clr_start} clr_len={clr_len}");
    }
    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < buf.len() {
        let val = buf[start];
        let mut end = start + 1;
        while end < buf.len() && buf[end] == val {
            end += 1;
        }
        ranges.push((start, end - 1, val));
        start = end;
    }
    println!("buffer ranges (relative indices): {ranges:?}");
}
