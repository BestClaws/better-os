use rust_gfx::*;
use rust_gfx::primitives::mask::RadiusMask;

fn main() {
    // Test case from border w1
    let outer_area = Area::new(20, 30, 82, 95);
    let inner_area = Area::new(21, 31, 81, 94);
    
    // Masks as used in border drawing:
    // - inner_mask: outer=true (keep outside inner rect, cut hole)
    // - outer_mask: outer=false (keep inside outer rect, rounded edges)
    let inner_mask = RadiusMask::new(inner_area, 9, true);
    let outer_mask = RadiusMask::new(outer_area, 10, false);
    
    // Test at y=30, which is the top edge
    let y = 30;
    let width = 63; // outer width
    let mut mask_buf = vec![255u8; width];
    
    println!("Initial mask_buf[0-10]: {:?}", &mask_buf[0..11]);
    
    // Apply inner mask ONLY (not outer)
    inner_mask.apply_to_line(y, 20, &mut mask_buf);
    println!("\nAfter inner mask ONLY:");
    println!("  mask_buf[0-10]: {:?}", &mask_buf[0..11]);
    println!("  mask_buf[6] at x=26: {}", mask_buf[6]);
    println!("\nExpected: 136, Getting: {}", mask_buf[6]);
    
    // Now also try with outer mask
    let mut mask_buf2 = vec![255u8; width];
    inner_mask.apply_to_line(y, 20, &mut mask_buf2);
    outer_mask.apply_to_line(y, 20, &mut mask_buf2);
    println!("\nWith BOTH masks:");
    println!("  mask_buf[6] at x=26: {}", mask_buf2[6]);
    
    // Expected: around 136 for anti-aliased border
    // Actual: we're getting 80
    
    println!("\nFull mask values at y=30:");
    for (i, &val) in mask_buf.iter().enumerate() {
        println!("  x={} (idx={}): {}", 20 + i, i, val);
    }
    
    // Let's also test what the CircleCache returns directly
    println!("\nCircleCache data for outer radius=10:");
    if let Some(cache) = outer_mask.get_cache() {
        for y_offset in 0..11 {
            if let Some(line_data) = cache.get_line_data(y_offset) {
                println!("y_offset={}: x_start={}, opa={:?}", 
                    y_offset, line_data.x_start, 
                    &line_data.opa);
            }
        }
    }
    
    // Test calculations manually
    println!("\nManual calculation for pixel x=26:");
    println!("- y=30, area.y1=30, rel_y=0");
    println!("- radius=10, cir_y = 10-0-1 = 9");
    println!("- CircleCache at cir_y=9: x_start=1, opa=[0, 80, 160, 224]");
    println!("- aa_len = 4");
    println!("- k = 20 - 20 = 0");
    println!("- w = 63, h = 66");
    println!("- cir_x_left = 0 + 10 - 1 - 1 = 8");
    println!("- For i=2: opa = 255 - opa[1] = 255 - 80 = 175");
    println!("- left_idx = 8 - 2 = 6");
    println!("- Expected: mask_mix(175, 255) = 175");
    println!("- Actual: {}", mask_buf[6]);
}
