use rust_gfx::masks::{apply_masks, AngleMask, MaskRef, RadiusMask};
use rust_gfx::types::Area;

fn main() {
    let center_x = 51;
    let center_y = 62;
    let radius = 35;
    let width = 3;

    let angle_mask = AngleMask::new(center_x, center_y, 0, 90);

    let outer_area = Area::new(center_x - radius, center_y - radius, center_x + radius, center_y + radius);
    let inner_area = Area::new(
        center_x - (radius - width),
        center_y - (radius - width),
        center_x + (radius - width),
        center_y + (radius - width),
    );

    let outer_mask = RadiusMask::new(outer_area, radius, false);
    let inner_mask = RadiusMask::new(inner_area, radius - width, true);

    let blend_area = outer_area;
    let draw_width = (blend_area.x2 - blend_area.x1 + 1) as usize;
    let masks = [
        MaskRef::Angle(&angle_mask),
        MaskRef::Radius(&outer_mask),
        MaskRef::Radius(&inner_mask),
    ];
    let radius_only = [MaskRef::Radius(&outer_mask)];
    let inner_only = [MaskRef::Radius(&inner_mask)];

    for y in 60..=64 {
        let mut buf = vec![255u8; draw_width];
        let res = apply_masks(&masks, &mut buf, blend_area.x1, y);
        let slice = &buf[..];
        let mut ranges = Vec::new();
        let mut current: Option<(usize, u8)> = None;
        for (idx, &val) in slice.iter().enumerate() {
            match current {
                Some((start, v)) if v == val => {}
                Some((start, v)) => {
                    ranges.push((start, idx - 1, v));
                    current = Some((idx, val));
                }
                None => current = Some((idx, val)),
            }
        }
        if let Some((start, v)) = current {
            ranges.push((start, slice.len() - 1, v));
        }
        println!(
            "y={} res={:?} ranges={:?} x coords {:?}",
            y,
            res,
            ranges,
            ranges
                .iter()
                .map(|(s, e, _)| (blend_area.x1 + *s as i32, blend_area.x1 + *e as i32))
                .collect::<Vec<_>>()
        );
        let mut buf_outer = vec![255u8; draw_width];
        let res_outer = apply_masks(&radius_only, &mut buf_outer, blend_area.x1, y);
        #[cfg(debug_assertions)]
        {
            let mut buf_angle = vec![255u8; draw_width];
            let res_angle = angle_mask.apply(&mut buf_angle, blend_area.x1, y);
            println!(
                "angle-only y={} res={:?} ranges={:?}",
                y,
                res_angle,
                compress_ranges(&buf_angle)
                    .iter()
                    .map(|(s, e, v)| (blend_area.x1 + *s as i32, blend_area.x1 + *e as i32, *v))
                    .collect::<Vec<_>>()
            );
            let ranges_outer = compress_ranges(&buf_outer);
            println!(
                "outer-only y={} res={:?} ranges={:?}",
                y,
                res_outer,
                ranges_outer
                    .iter()
                    .map(|(s, e, v)| (blend_area.x1 + *s as i32, blend_area.x1 + *e as i32, *v))
                    .collect::<Vec<_>>()
            );

            let mut buf_inner = vec![255u8; draw_width];
            let res_inner = apply_masks(&inner_only, &mut buf_inner, blend_area.x1, y);
            let ranges_inner = compress_ranges(&buf_inner);
            println!(
                "inner-only y={} res={:?} ranges={:?}",
                y,
                res_inner,
                ranges_inner
                    .iter()
                    .map(|(s, e, v)| (blend_area.x1 + *s as i32, blend_area.x1 + *e as i32, *v))
                    .collect::<Vec<_>>()
            );

            let rel_y_outer = y - outer_area.y1;
            let rel_y_inner = y - inner_area.y1;
            if let Some((x_start, slice)) = inner_mask.debug_line_info(rel_y_inner) {
                let aa_len = slice.len();
                let k = inner_area.x1 - blend_area.x1;
                let w = inner_area.x2 - inner_area.x1 + 1;
                let cir_x_right = k + w - (radius - width) + x_start;
                let cir_x_left = k + (radius - width) - x_start - 1;
                let clr_start = (cir_x_left + 1).clamp(0, draw_width as i32);
                let clr_end = clr_start.clamp(cir_x_right, draw_width as i32);
                println!(
                    "    inner debug rel_y={} x_start={} aa_len={} k={} w={} cir_left={} cir_right={} clr={}..{}",
                    rel_y_inner, x_start, aa_len, k, w, cir_x_left, cir_x_right, clr_start, clr_end
                );
            }
        }
        #[cfg(debug_assertions)]
        {
            let rel_y = y - outer_area.y1;
            if let Some((x_start, values)) = outer_mask.debug_line_info(rel_y) {
                println!("outer rel_y={} x_start={} opa={:?}", rel_y, x_start, values);
            }
            if let Some((x_start, values)) = inner_mask.debug_line_info(y - inner_area.y1) {
                println!("inner rel_y={} x_start={} opa={:?}", y - inner_area.y1, x_start, values);
            }
        }
    }
}

#[cfg(debug_assertions)]
fn compress_ranges(buf: &[u8]) -> Vec<(usize, usize, u8)> {
    let mut ranges = Vec::new();
    let mut current: Option<(usize, u8)> = None;
    for (idx, &val) in buf.iter().enumerate() {
        match current {
            Some((start, v)) if v == val => {}
            Some((start, v)) => {
                ranges.push((start, idx - 1, v));
                current = Some((idx, val));
            }
            None => current = Some((idx, val)),
        }
    }
    if let Some((start, v)) = current {
        ranges.push((start, buf.len() - 1, v));
    }
    ranges
}
