/// Arc drawing matching LVGL's lv_draw_arc
use alloc::vec;
use alloc::vec::Vec;

use crate::color::Rgba8888;
use crate::masks::{apply_masks, AngleMask, MaskRef, MaskResult, RadiusMask};
use crate::types::*;
use crate::Rasterizer;

/// Arc descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct ArcDsc {
    pub center: Point,
    pub radius: i32,
    pub start_angle: i32, // degrees
    pub end_angle: i32,   // degrees
    pub width: i32,
    pub color: Rgba8888,
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
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            rounded: false,
        }
    }
}

/// Draw an arc using mask-based rendering (matching LVGL)
pub fn draw_arc<R: Rasterizer>(rast: &mut R, dsc: &ArcDsc) {
    if dsc.width == 0 {
        return;
    }

    let cx = dsc.center.x;
    let cy = dsc.center.y;
    let width = dsc.width.min(dsc.radius);

    // Calculate inner and outer radii
    let r_outer = dsc.radius;
    let r_inner = dsc.radius - width;

    // Normalize angles to 0-359
    let mut start_angle = dsc.start_angle;
    let mut end_angle = dsc.end_angle;
    while start_angle >= 360 {
        start_angle -= 360;
    }
    while end_angle >= 360 {
        end_angle -= 360;
    }

    // Create angle mask
    let angle_mask = AngleMask::new(cx, cy, start_angle, end_angle);

    // Create radius masks (outer and inner)
    let outer_area = Area::new(cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer);
    let inner_area = Area::new(cx - r_inner, cy - r_inner, cx + r_inner, cy + r_inner);

    let mask_outer = RadiusMask::new(outer_area, r_outer, false); // Keep inside
    let mask_inner = RadiusMask::new(inner_area, r_inner, true); // Keep outside

    // Calculate blend area
    let blend_area = Area::new(cx - r_outer, cy - r_outer, cx + r_outer, cy + r_outer);

    // Draw arc using masks
    let draw_width = (blend_area.x2 - blend_area.x1 + 1) as usize;
    let mut mask_buf = vec![255u8; draw_width];

    for y in blend_area.y1..=blend_area.y2 {
        // Reset mask buffer
        mask_buf.fill(255);

        // Apply masks
        let masks = vec![
            MaskRef::Angle(&angle_mask),
            MaskRef::Radius(&mask_outer),
            MaskRef::Radius(&mask_inner),
        ];
        let res = apply_masks(&masks, &mut mask_buf, blend_area.x1, y);

        if res == MaskResult::Transparent {
            continue;
        }

        // Blend pixels
        for (i, &opa) in mask_buf.iter().enumerate() {
            if opa > 0 {
                let x = blend_area.x1 + i as i32;
                let final_opa = ((dsc.opa as u32 * opa as u32) / 255) as Opa;
                rast.blend_pixel(x, y, dsc.color, final_opa);
            }
        }
    }

    rast.mark_dirty(
        blend_area.x1,
        blend_area.y1,
        blend_area.x2 + 1,
        blend_area.y2 + 1,
    );
}
