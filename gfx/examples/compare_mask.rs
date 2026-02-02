use zeno::{Angle, ArcSize, ArcSweep, Fill as ZenoFill, Mask, Origin, PathBuilder, Point, Vector};

fn main() {
    let x0 = 2.0;
    let y0 = 2.0;
    let x1 = 18.0;
    let y1 = 18.0;
    let mut path = Vec::new();
    let rx = 6.0;
    let ry = 6.0;
    path.move_to((x0 + rx, y0));
    path.line_to((x1 - rx, y0));
    path.arc_to(rx, ry, Angle::ZERO, ArcSize::Small, ArcSweep::Positive, Point::new(x1, y0 + ry));
    path.line_to((x1, y1 - ry));
    path.arc_to(rx, ry, Angle::ZERO, ArcSize::Small, ArcSweep::Positive, Point::new(x1 - rx, y1));
    path.line_to((x0 + rx, y1));
    path.arc_to(rx, ry, Angle::ZERO, ArcSize::Small, ArcSweep::Positive, Point::new(x0, y1 - ry));
    path.line_to((x0, y0 + ry));
    path.arc_to(rx, ry, Angle::ZERO, ArcSize::Small, ArcSweep::Positive, Point::new(x0 + rx, y0));
    path.close();

    let mut mask_full = Mask::new(&path);
    mask_full.style(ZenoFill::NonZero);
    mask_full.origin(Origin::TopLeft);
    mask_full.offset(Vector::new(-2.0, -2.0));
    mask_full.size(16, 16);
    mask_full.render_offset(Vector::new(0.0, -0.5));
    let (mut full, _) = mask_full.render();
    let width = 16usize;
    let height = 16usize;
    let mut adjusted = full.clone();
    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            let value = full[idx];
            if value == 0 {
                let mut neighbor = false;
                if col > 0 && full[idx - 1] > 0 { neighbor = true; }
                if col + 1 < width && full[idx + 1] > 0 { neighbor = true; }
                if row > 0 && full[idx - width] > 0 { neighbor = true; }
                if row + 1 < height && full[idx + width] > 0 { neighbor = true; }
                if neighbor {
                    adjusted[idx] = 1;
                }
            } else if value < 255 {
                adjusted[idx] = value.saturating_add(1);
            }
        }
    }

    let mut mismatch = 0;
    let mut max_delta = 0i32;
    let mut min_delta = 0i32;
    for row in 0..16 {
        let mut mask_row = Mask::new(&path);
        mask_row.style(ZenoFill::NonZero);
        mask_row.origin(Origin::TopLeft);
        mask_row.offset(Vector::new(-2.0, -(2.0 + row as f32)));
        mask_row.size(16, 1);
        mask_row.render_offset(Vector::new(0.0, 0.0));
        let (row_buf, _) = mask_row.render();
        for col in 0..16 {
            let full_value = full[row * 16 + col];
            let adjusted_value = adjusted[row * 16 + col];
            let row_value = row_buf[col];
            if adjusted_value != row_value {
                println!("diff at row {}, col {}: adjusted={}, row={} (orig={})", row, col, adjusted_value, row_value, full_value);
                let idx = row * 16 + col;
                println!("  neighbors full: left={}, right={}, up={}, down={}",
                    if col > 0 { full[idx - 1] } else { 0 },
                    if col + 1 < 16 { full[idx + 1] } else { 0 },
                    if row > 0 { full[idx - 16] } else { 0 },
                    if row + 1 < 16 { full[idx + 16] } else { 0 },
                );
                println!("  neighbors row: left={}, right={}, up={}, down={}",
                    if col > 0 { row_buf[col - 1] } else { 0 },
                    if col + 1 < 16 { row_buf[col + 1] } else { 0 },
                    0,
                    0,
                );
                let delta = adjusted_value as i32 - row_value as i32;
                if delta > max_delta {
                    max_delta = delta;
                }
                if delta < min_delta {
                    min_delta = delta;
                }
                mismatch += 1;
                if mismatch >= 32 {
                    println!("min_delta={}, max_delta={}", min_delta, max_delta);
                    return;
                }
            }
        }
    }
    if mismatch == 0 {
        println!("no diff");
    } else {
        println!("min_delta={}, max_delta={}", min_delta, max_delta);
    }
}
