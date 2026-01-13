use rust_gfx::masks::{apply_masks, AngleMask, MaskRef, MaskResult, RadiusMask};
use rust_gfx::types::Area;

struct ArcCase {
    start_angle: i32,
    end_angle: i32,
    width: i32,
    label: &'static str,
}

const ARC_CASES: [ArcCase; 16] = [
    ArcCase {
        start_angle: 0,
        end_angle: 90,
        width: 3,
        label: "arc_quarter_w3_a0",
    },
    ArcCase {
        start_angle: 90,
        end_angle: 180,
        width: 3,
        label: "arc_quarter_w3_a90",
    },
    ArcCase {
        start_angle: 180,
        end_angle: 270,
        width: 3,
        label: "arc_quarter_w3_a180",
    },
    ArcCase {
        start_angle: 270,
        end_angle: 360,
        width: 3,
        label: "arc_quarter_w3_a270",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 90,
        width: 8,
        label: "arc_quarter_w8_a0",
    },
    ArcCase {
        start_angle: 90,
        end_angle: 180,
        width: 8,
        label: "arc_quarter_w8_a90",
    },
    ArcCase {
        start_angle: 180,
        end_angle: 270,
        width: 8,
        label: "arc_quarter_w8_a180",
    },
    ArcCase {
        start_angle: 270,
        end_angle: 360,
        width: 8,
        label: "arc_quarter_w8_a270",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 90,
        width: 15,
        label: "arc_quarter_w15_a0",
    },
    ArcCase {
        start_angle: 90,
        end_angle: 180,
        width: 15,
        label: "arc_quarter_w15_a90",
    },
    ArcCase {
        start_angle: 180,
        end_angle: 270,
        width: 15,
        label: "arc_quarter_w15_a180",
    },
    ArcCase {
        start_angle: 270,
        end_angle: 360,
        width: 15,
        label: "arc_quarter_w15_a270",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 45,
        width: 8,
        label: "arc_span45",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 90,
        width: 8,
        label: "arc_span90",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 180,
        width: 8,
        label: "arc_span180",
    },
    ArcCase {
        start_angle: 0,
        end_angle: 270,
        width: 8,
        label: "arc_span270",
    },
];

fn mask_res_to_str(res: MaskResult) -> &'static str {
    match res {
        MaskResult::Transparent => "Transparent",
        MaskResult::FullCover => "FullCover",
        MaskResult::Changed => "Changed",
    }
}

fn main() {
    let center_x = 51;
    let center_y = 62;
    let radius = 35;
    let outer_area = Area::new(
        center_x - radius,
        center_y - radius,
        center_x + radius,
        center_y + radius,
    );
    let draw_width = (outer_area.x2 - outer_area.x1 + 1) as usize;
    let mut first_case = true;

    for (case_idx, case) in ARC_CASES.iter().enumerate() {
        if case_idx > 0 {
            println!();
        }
        println!(
            "case={} start={} end={} width={}",
            case.label, case.start_angle, case.end_angle, case.width
        );

        let angle_mask = AngleMask::new(center_x, center_y, case.start_angle, case.end_angle);
        let outer_mask = RadiusMask::new(outer_area, radius, false);

        let inner_radius = (radius - case.width).max(0);
        let inner_area = Area::new(
            center_x - inner_radius,
            center_y - inner_radius,
            center_x + inner_radius,
            center_y + inner_radius,
        );
        let inner_mask = RadiusMask::new(inner_area, inner_radius, true);

        let masks = [
            MaskRef::Angle(&angle_mask),
            MaskRef::Radius(&outer_mask),
            MaskRef::Radius(&inner_mask),
        ];
        let outer_only = [MaskRef::Radius(&outer_mask)];
        let inner_only = [MaskRef::Radius(&inner_mask)];

        for y in outer_area.y1..=outer_area.y2 {
            let mut buf = vec![255u8; draw_width];
            let res = apply_masks(&masks, &mut buf, outer_area.x1, y);
            if res == MaskResult::Transparent {
                buf.fill(0);
            }
            let ranges = compress_ranges(&buf)
                .iter()
                .map(|(s, e, v)| (outer_area.x1 + *s as i32, outer_area.x1 + *e as i32, *v))
                .collect::<Vec<_>>();
            println!(
                "combined case={} y={} res={} ranges={:?}",
                case.label,
                y,
                mask_res_to_str(res),
                ranges
            );

            #[cfg(debug_assertions)]
            if first_case && (60..=64).contains(&y) {
                let mut buf_angle = vec![255u8; draw_width];
                let res_angle = angle_mask.apply(&mut buf_angle, outer_area.x1, y);
                if res_angle == MaskResult::Transparent {
                    buf_angle.fill(0);
                }
                println!(
                    "angle-only case={} y={} res={} ranges={:?}",
                    case.label,
                    y,
                    mask_res_to_str(res_angle),
                    compress_ranges(&buf_angle)
                        .iter()
                        .map(|(s, e, v)| (outer_area.x1 + *s as i32, outer_area.x1 + *e as i32, *v))
                        .collect::<Vec<_>>()
                );

                let mut buf_outer = vec![255u8; draw_width];
                let res_outer = apply_masks(&outer_only, &mut buf_outer, outer_area.x1, y);
                if res_outer == MaskResult::Transparent {
                    buf_outer.fill(0);
                }
                println!(
                    "outer-only case={} y={} res={} ranges={:?}",
                    case.label,
                    y,
                    mask_res_to_str(res_outer),
                    compress_ranges(&buf_outer)
                        .iter()
                        .map(|(s, e, v)| (outer_area.x1 + *s as i32, outer_area.x1 + *e as i32, *v))
                        .collect::<Vec<_>>()
                );

                let mut buf_inner = vec![255u8; draw_width];
                let res_inner = apply_masks(&inner_only, &mut buf_inner, outer_area.x1, y);
                if res_inner == MaskResult::Transparent {
                    buf_inner.fill(0);
                }
                println!(
                    "inner-only case={} y={} res={} ranges={:?}",
                    case.label,
                    y,
                    mask_res_to_str(res_inner),
                    compress_ranges(&buf_inner)
                        .iter()
                        .map(|(s, e, v)| (outer_area.x1 + *s as i32, outer_area.x1 + *e as i32, *v))
                        .collect::<Vec<_>>()
                );

                let rel_y_outer = y - outer_area.y1;
                if let Some((x_start, values)) = outer_mask.debug_line_info(rel_y_outer) {
                    println!(
                        "outer case={} rel_y={} x_start={} opa={:?}",
                        case.label, rel_y_outer, x_start, values
                    );
                }
                let rel_y_inner = y - inner_area.y1;
                if let Some((x_start, values)) = inner_mask.debug_line_info(rel_y_inner) {
                    println!(
                        "inner case={} rel_y={} x_start={} opa={:?}",
                        case.label, rel_y_inner, x_start, values
                    );
                }
            }
        }

        first_case = false;
    }
}

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
