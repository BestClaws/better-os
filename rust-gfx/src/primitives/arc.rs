/// Arc drawing matching LVGL's lv_draw_arc
use crate::canvas::Canvas;
use crate::color_argb::Argb8888;
use crate::types::*;
use crate::math::{aa_coverage_sq, atan2_deg, angle_in_range, dist_sq};

/// Arc descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct ArcDsc {
    pub center: Point,
    pub radius: i32,
    pub start_angle: i32,  // degrees
    pub end_angle: i32,    // degrees
    pub width: i32,
    pub color: Argb8888,
    pub opa: Opa,
    pub rounded: bool,
}

impl ArcDsc {
    pub fn new(center: Point, radius: i32, start_angle: i32, end_angle: i32) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            width: 1,
            color: Argb8888::WHITE,
            opa: OPA_COVER,
            rounded: false,
        }
    }
}

/// Draw an arc
pub fn draw_arc(canvas: &mut Canvas, dsc: &ArcDsc) {
    let cx = dsc.center.x;
    let cy = dsc.center.y;
    let r_outer = dsc.radius + dsc.width / 2;
    let r_inner = dsc.radius - dsc.width / 2;

    let min_x = cx - r_outer;
    let max_x = cx + r_outer;
    let min_y = cy - r_outer;
    let max_y = cy + r_outer;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x - cx;
            let dy = y - cy;
            let d_sq = dx * dx + dy * dy;

            // Check if in radius range
            if d_sq < r_inner * r_inner || d_sq > r_outer * r_outer {
                continue;
            }

            // Check angle
            let angle = atan2_deg(dy, dx);
            if !angle_in_range(angle, dsc.start_angle, dsc.end_angle) {
                continue;
            }

            // Calculate coverage
            let coverage = if d_sq <= dsc.radius * dsc.radius {
                // Inner part
                aa_coverage_sq(d_sq, r_inner)
            } else {
                // Outer part
                255 - aa_coverage_sq(d_sq, r_outer)
            };

            if coverage > 0 {
                let opa = ((dsc.opa as u32 * coverage as u32) / 255) as Opa;
                canvas.blend_pixel(x, y, dsc.color, opa);
            }
        }
    }
}
