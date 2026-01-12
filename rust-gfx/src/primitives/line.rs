/// Line drawing matching LVGL's lv_draw_line
use crate::Rasterizer;
use crate::color::Rgba8888;
use crate::types::*;
use crate::math::{aa_coverage_sq, dist_sq};

/// Line descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct LineDsc {
    pub p1: Point,
    pub p2: Point,
    pub width: i32,
    pub color: Rgba8888,
    pub opa: Opa,
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
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
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
    
    if (is_horizontal || is_vertical) && !dsc.round_start && !dsc.round_end && dsc.dash_width == 0 {
        // Draw as filled rectangle (matching LVGL's draw_line_hor/draw_line_ver)
        let w = dsc.width - 1;
        let w_half0 = w / 2;
        let w_half1 = w_half0 + (w & 1); // Compensate for odd width
        
        let (x1, x2, y1, y2) = if is_horizontal {
            (dsc.p1.x.min(dsc.p2.x), 
             dsc.p1.x.max(dsc.p2.x) - 1,  // LVGL subtracts 1 from max coordinate
             dsc.p1.y - w_half1,
             dsc.p1.y + w_half0)
        } else {
            (dsc.p1.x - w_half1,
             dsc.p1.x + w_half0,
             dsc.p1.y.min(dsc.p2.y),
             dsc.p1.y.max(dsc.p2.y) - 1)  // LVGL subtracts 1 from max coordinate
        };
        
        // Draw filled rectangle
        for y in y1..=y2 {
            for x in x1..=x2 {
                rast.blend_pixel(x, y, dsc.color, dsc.opa);
            }
        }
        return;
    }
    
    // General case: antialiased line with distance-based rendering
    let len_sq = dx * dx + dy * dy;
    
    if len_sq == 0 {
        // Point
        rast.blend_pixel(dsc.p1.x, dsc.p1.y, dsc.color, dsc.opa);
        return;
    }

    let min_x = dsc.p1.x.min(dsc.p2.x) - dsc.width;
    let max_x = dsc.p1.x.max(dsc.p2.x) + dsc.width;
    let min_y = dsc.p1.y.min(dsc.p2.y) - dsc.width;
    let max_y = dsc.p1.y.max(dsc.p2.y) + dsc.width;

    let half_width = dsc.width / 2;
    let is_dashed = dsc.dash_width > 0 && dsc.dash_gap > 0;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            // Calculate perpendicular distance to line
            let px = x - dsc.p1.x;
            let py = y - dsc.p1.y;
            let t = ((px * dx + py * dy) * 256) / len_sq;

            let (d_sq, on_line) = if t < 0 {
                (dist_sq(x, y, dsc.p1.x, dsc.p1.y), dsc.round_start)
            } else if t > 256 {
                (dist_sq(x, y, dsc.p2.x, dsc.p2.y), dsc.round_end)
            } else {
                let proj_x = dsc.p1.x + (dx * t) / 256;
                let proj_y = dsc.p1.y + (dy * t) / 256;
                (dist_sq(x, y, proj_x, proj_y), true)
            };

            if !on_line {
                continue;
            }

            // Check dash pattern
            if is_dashed {
                let line_pos = ((t * crate::math::isqrt(len_sq as u32) as i32) / 256).max(0);
                let pattern_len = dsc.dash_width + dsc.dash_gap;
                let pos_in_pattern = line_pos % pattern_len;
                if pos_in_pattern >= dsc.dash_width {
                    continue;
                }
            }

            let coverage = aa_coverage_sq(d_sq, half_width);
            if coverage > 0 {
                let opa = ((dsc.opa as u32 * coverage as u32) / 255) as Opa;
                rast.blend_pixel(x, y, dsc.color, opa);
            }
        }
    }
}
