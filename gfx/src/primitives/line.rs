/// Line drawing matching LVGL's lv_draw_line
use alloc::vec;
use alloc::vec::Vec;

use crate::colors::Color;
use crate::masks::{apply_masks, LineMask, LineSide, MaskRef, MaskResult};
use crate::primitives::rectangle::{draw_rect, RectDsc};
use crate::types::*;
use crate::Rasterizer;

fn pos_mod(value: i32, modulo: i32) -> i32 {
    let mut result = value % modulo;
    if result < 0 {
        result += modulo;
    }
    result
}

/// Line descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct LineDsc {
    pub p1: Point,
    pub p2: Point,
    pub width: i32,
    pub color: Color,
    pub opa: Opacity,
    pub dash_width: i32,
    pub dash_gap: i32,
    pub round_start: bool,
    pub round_end: bool,
}

impl LineDsc {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self {
            p1,
            p2,
            width: 1,
            color: Color::WHITE,
            opa: OPA100,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        }
    }
}

/// Draw a line
pub fn draw_line<R: Rasterizer>(rast: &mut R, dsc: &LineDsc) {
    let dx = dsc.p2.x - dsc.p1.x;
    let dy = dsc.p2.y - dsc.p1.y;

    // LVGL optimization: horizontal and vertical lines use rectangle drawing
    let is_horizontal = dy == 0 && dx != 0;
    let is_vertical = dx == 0 && dy != 0;
    let dashed = dsc.dash_width > 0 && dsc.dash_gap > 0;

    if dashed {
        if is_horizontal {
            draw_horizontal_dashed(rast, dsc);
            return;
        }

        if is_vertical {
            draw_vertical_dashed(rast, dsc);
            return;
        }
    }

    if (is_horizontal || is_vertical) && dsc.dash_width == 0 {
        // Draw as filled rectangle (matching LVGL's draw_line_hor/draw_line_ver)
        let w = dsc.width - 1;
        let w_half0 = w / 2;
        let w_half1 = w_half0 + (w & 1); // Compensate for odd width

        let (x1, x2, y1, y2) = if is_horizontal {
            (
                dsc.p1.x.min(dsc.p2.x),
                dsc.p1.x.max(dsc.p2.x) - 1, // LVGL subtracts 1 from max coordinate
                dsc.p1.y - w_half1,
                dsc.p1.y + w_half0,
            )
        } else {
            (
                dsc.p1.x - w_half1,
                dsc.p1.x + w_half0,
                dsc.p1.y.min(dsc.p2.y),
                dsc.p1.y.max(dsc.p2.y) - 1,
            ) // LVGL subtracts 1 from max coordinate
        };

        // Draw filled rectangle
        if x1 <= x2 && y1 <= y2 {
            let width = x2 - x1 + 1;
            let height = y2 - y1 + 1;
            if dsc.opa == OPA100 {
                rast.fill_rect(x1, y1, width, height, dsc.color);
            } else {
                for y in y1..=y2 {
                    rast.blend_hspan_with(x1, y, width, |_| (dsc.color, dsc.opa));
                }
            }
        }
        rast.mark_dirty(x1, y1, x2 + 1, y2 + 1);
        return;
    }

    // General case: diagonal line with mask-based rendering (matching LVGL)
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0 {
        // Point
        rast.blend_pixel(dsc.p1.x, dsc.p1.y, dsc.color, dsc.opa);
        rast.mark_dirty(dsc.p1.x, dsc.p1.y, dsc.p1.x + 1, dsc.p1.y + 1);
        return;
    }

    // Use LVGL's width correction for diagonal lines
    let xdiff = dsc.p2.x - dsc.p1.x;
    let ydiff = dsc.p2.y - dsc.p1.y;
    let flat = xdiff.abs() > ydiff.abs();

    // LVGL's width correction table
    const WCORR: [u16; 33] = [
        128, 128, 128, 129, 129, 130, 130, 131, 132, 133, 134, 135, 137, 138, 140, 141, 143, 145,
        147, 149, 151, 153, 155, 158, 160, 162, 165, 167, 170, 173, 175, 178, 181,
    ];

    let mut w = dsc.width;
    let wcorr_i = if flat {
        ((ydiff.abs() << 5) / xdiff.abs()).min(32)
    } else {
        ((xdiff.abs() << 5) / ydiff.abs()).min(32)
    } as usize;
    w = ((w as i32 * WCORR[wcorr_i] as i32 + 63) >> 7) as i32;

    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    // Determine point order (left to right for flat, top to bottom for steep)
    let (p1, p2) = if flat {
        if xdiff > 0 {
            (dsc.p1, dsc.p2)
        } else {
            (dsc.p2, dsc.p1)
        }
    } else {
        if ydiff > 0 {
            (dsc.p1, dsc.p2)
        } else {
            (dsc.p2, dsc.p1)
        }
    };

    // Create line masks for the edges
    let (mask_left, mask_right) = if flat {
        let xdiff_ordered = p2.x - p1.x;
        if xdiff_ordered > 0 {
            (
                LineMask::from_points(
                    Point::new(p1.x, p1.y - w_half0),
                    Point::new(p2.x, p2.y - w_half0),
                    LineSide::Left,
                ),
                LineMask::from_points(
                    Point::new(p1.x, p1.y + w_half1),
                    Point::new(p2.x, p2.y + w_half1),
                    LineSide::Right,
                ),
            )
        } else {
            (
                LineMask::from_points(
                    Point::new(p1.x, p1.y + w_half1),
                    Point::new(p2.x, p2.y + w_half1),
                    LineSide::Left,
                ),
                LineMask::from_points(
                    Point::new(p1.x, p1.y - w_half0),
                    Point::new(p2.x, p2.y - w_half0),
                    LineSide::Right,
                ),
            )
        }
    } else {
        (
            LineMask::from_points(
                Point::new(p1.x + w_half1, p1.y),
                Point::new(p2.x + w_half1, p2.y),
                LineSide::Left,
            ),
            LineMask::from_points(
                Point::new(p1.x - w_half0, p1.y),
                Point::new(p2.x - w_half0, p2.y),
                LineSide::Right,
            ),
        )
    };

    // End cap masks (LVGL always clips line length unless raw_end is requested)
    let ydiff_ordered = p2.y - p1.y;
    let xdiff_ordered = p2.x - p1.x;
    let mask_top = LineMask::from_points(
        p1,
        Point::new(p1.x - ydiff_ordered, p1.y + xdiff_ordered),
        LineSide::Bottom,
    );
    let mask_bottom = LineMask::from_points(
        p2,
        Point::new(p2.x - ydiff_ordered, p2.y + xdiff_ordered),
        LineSide::Top,
    );

    // Calculate blend area
    let blend_area = Area::new(
        p1.x.min(p2.x) - w,
        p1.y.min(p2.y) - w,
        p1.x.max(p2.x) + w,
        p1.y.max(p2.y) + w,
    );

    // Draw line using masks
    let draw_width = blend_area.x2 - blend_area.x1 + 1;
    let draw_width_usize = draw_width as usize;
    let mut mask_buf = vec![255u8; draw_width_usize];
    let mut coverage_row = vec![0u8; draw_width_usize];
    let masks = [
        MaskRef::Line(&mask_left),
        MaskRef::Line(&mask_right),
        MaskRef::Line(&mask_top),
        MaskRef::Line(&mask_bottom),
    ];

    for y in blend_area.y1..=blend_area.y2 {
        // Reset mask buffer
        mask_buf.fill(255);
        coverage_row.fill(0);

        // Apply masks
        let res = apply_masks(&masks, &mut mask_buf, blend_area.x1, y);

        if res == MaskResult::Transparent {
            continue;
        }

        if res == MaskResult::FullCover && dsc.opa == OPA100 {
            rast.fill_rect(blend_area.x1, y, draw_width, 1, dsc.color);
            continue;
        }

        let mut any = false;
        for (i, &opa) in mask_buf.iter().enumerate() {
            let final_opa = ((dsc.opa as u32 * opa as u32) / 255) as Opacity;
            coverage_row[i] = final_opa;
            if final_opa != 0 {
                any = true;
            }
        }

        if any {
            rast.blend_solid_hspan(blend_area.x1, y, dsc.color, &coverage_row);
        }
    }

    // Mark dirty region
    rast.mark_dirty(
        blend_area.x1,
        blend_area.y1,
        blend_area.x2 + 1,
        blend_area.y2 + 1,
    );

    if dsc.round_start || dsc.round_end {
        let mut cap_dsc = RectDsc::new();
        cap_dsc.bg_color = dsc.color;
        cap_dsc.bg_opa = dsc.opa;
        cap_dsc.radius = RADIUS_CIRCLE;

        let radius = dsc.width >> 1;
        let r_corr = if dsc.width & 1 == 0 { 1 } else { 0 };

        if dsc.round_start && dsc.width > 0 {
            let area = Area::new(
                dsc.p1.x - radius,
                dsc.p1.y - radius,
                dsc.p1.x + radius - r_corr,
                dsc.p1.y + radius - r_corr,
            );
            draw_rect(rast, &cap_dsc, &area);
        }

        if dsc.round_end && dsc.width > 0 {
            let area = Area::new(
                dsc.p2.x - radius,
                dsc.p2.y - radius,
                dsc.p2.x + radius - r_corr,
                dsc.p2.y + radius - r_corr,
            );
            draw_rect(rast, &cap_dsc, &area);
        }
    }
}

fn draw_horizontal_dashed<R: Rasterizer>(rast: &mut R, dsc: &LineDsc) {
    let w = dsc.width - 1;
    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    let x_start = dsc.p1.x.min(dsc.p2.x);
    let x_end = dsc.p1.x.max(dsc.p2.x) - 1;
    let y_start = dsc.p1.y - w_half1;
    let y_end = dsc.p1.y + w_half0;

    if x_end < x_start || y_end < y_start {
        return;
    }

    let pattern = dsc.dash_width + dsc.dash_gap;
    if pattern <= 0 {
        return;
    }

    let dash_start = pos_mod(x_start, pattern);
    let width = (x_end - x_start + 1) as usize;
    let mut coverage_row = vec![0u8; width];

    for y in y_start..=y_end {
        coverage_row.fill(0);
        let mut any = false;
        for offset in 0..width {
            let dash_cnt = (dash_start + offset as i32) % pattern;
            let x = x_start + offset as i32;

            if dash_cnt < dsc.dash_width {
                coverage_row[offset] = dsc.opa;
                any = any || dsc.opa != 0;
            } else {
                rast.stamp_rgb_zero_alpha(x, y, dsc.color);
            }
        }

        if any {
            rast.blend_solid_hspan(x_start, y, dsc.color, &coverage_row);
        }
    }

    rast.mark_dirty(x_start, y_start, x_end + 1, y_end + 1);
}

fn draw_vertical_dashed<R: Rasterizer>(rast: &mut R, dsc: &LineDsc) {
    let w = dsc.width - 1;
    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    let x_start = dsc.p1.x - w_half1;
    let x_end = dsc.p1.x + w_half0;
    let y_start = dsc.p1.y.min(dsc.p2.y);
    let y_end = dsc.p1.y.max(dsc.p2.y) - 1;

    if x_end < x_start || y_end < y_start {
        return;
    }

    let pattern = dsc.dash_width + dsc.dash_gap;
    if pattern <= 0 {
        return;
    }

    let mut dash_cnt = pos_mod(y_start, pattern);
    let span_len = x_end - x_start + 1;

    for y in y_start..=y_end {
        if dash_cnt > dsc.dash_width {
            for x in x_start..=x_end {
                rast.stamp_rgb_zero_alpha(x, y, dsc.color);
            }
        } else if dsc.opa == OPA100 {
            rast.fill_rect(x_start, y, span_len, 1, dsc.color);
        } else {
            rast.blend_hspan_with(x_start, y, span_len, |_| (dsc.color, dsc.opa));
        }

        dash_cnt += 1;
        if dash_cnt >= pattern {
            dash_cnt = 0;
        }
    }

    rast.mark_dirty(x_start, y_start, x_end + 1, y_end + 1);
}
