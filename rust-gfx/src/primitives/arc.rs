/// Arc drawing matching LVGL's lv_draw_arc
use alloc::vec;
use alloc::vec::Vec;

use crate::color::Rgba8888;
use crate::masks::{apply_masks, AngleMask, MaskRef, MaskResult, RadiusMask};
use crate::math::{trigo_cos, trigo_sin};
use crate::primitives::common::MaskBuffer;
use crate::types::*;
use crate::Rasterizer;

const LV_TRIGO_SHIFT: i32 = 15;

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
pub fn draw_arc<R>(rast: &mut R, dsc: &ArcDsc)
where
    R: Rasterizer,
{
    if dsc.opa == OPA_TRANSP {
        return;
    }
    if dsc.width == 0 {
        return;
    }

    let cx = dsc.center.x;
    let cy = dsc.center.y;
    let width = dsc.width.min(dsc.radius);

    let mut start_angle = dsc.start_angle;
    let mut end_angle = dsc.end_angle;
    while start_angle >= 360 {
        start_angle -= 360;
    }
    while end_angle >= 360 {
        end_angle -= 360;
    }

    let r_outer = dsc.radius;
    if r_outer <= 0 {
        return;
    }

    let area_out = Area::new(
        cx - r_outer,
        cy - r_outer,
        cx + r_outer - 1,
        cy + r_outer - 1,
    );
    let mut area_in = area_out;
    area_in.x1 += width;
    area_in.y1 += width;
    area_in.x2 -= width;
    area_in.y2 -= width;

    let angle_mask = AngleMask::new(cx, cy, start_angle, end_angle);
    let mask_outer = RadiusMask::new(area_out, RADIUS_CIRCLE, false);
    let mask_inner_opt = if area_in.x1 <= area_in.x2 && area_in.y1 <= area_in.y2 {
        Some(RadiusMask::new(area_in, RADIUS_CIRCLE, true))
    } else {
        None
    };

    let mut masks: Vec<MaskRef> = Vec::new();
    masks.push(MaskRef::Angle(&angle_mask));
    masks.push(MaskRef::Radius(&mask_outer));
    if let Some(inner) = mask_inner_opt.as_ref() {
        masks.push(MaskRef::Radius(inner));
    }

    let mut circle_mask = None;
    let mut round_area_start = None;
    let mut round_area_end = None;

    if dsc.rounded && width > 0 {
        let mask = build_circle_mask(width);
        let mut start_area = get_rounded_area(start_angle, r_outer, width);
        start_area.x1 += cx;
        start_area.x2 += cx;
        start_area.y1 += cy;
        start_area.y2 += cy;

        let mut end_area = get_rounded_area(end_angle, r_outer, width);
        end_area.x1 += cx;
        end_area.x2 += cx;
        end_area.y1 += cy;
        end_area.y2 += cy;

        circle_mask = Some(mask);
        round_area_start = Some(start_area);
        round_area_end = Some(end_area);
    }

    let mut row_area = area_out;
    let row_width = row_area.width().max(0) as usize;
    if row_width == 0 {
        return;
    }

    let mut mask_buffer = MaskBuffer::default();
    for y in area_out.y1..=area_out.y2 {
        row_area.y1 = y;
        row_area.y2 = y;

        let mask_slice = mask_buffer.prepare(row_width);
        let mut mask_res = apply_masks(&masks, mask_slice, row_area.x1, y);

        if let Some(circle) = circle_mask.as_ref() {
            if let Some(area_start) = round_area_start.as_ref() {
                if y >= area_start.y1 && y <= area_start.y2 {
                    if mask_res == MaskResult::Transparent {
                        mask_slice.fill(OPA_TRANSP);
                        mask_res = MaskResult::Changed;
                    }
                    add_circle(circle, width, &row_area, area_start, mask_slice);
                }
            }
            if let Some(area_end) = round_area_end.as_ref() {
                if y >= area_end.y1 && y <= area_end.y2 {
                    if mask_res == MaskResult::Transparent {
                        mask_slice.fill(OPA_TRANSP);
                        mask_res = MaskResult::Changed;
                    }
                    add_circle(circle, width, &row_area, area_end, mask_slice);
                }
            }
        }

        if mask_res == MaskResult::Transparent {
            continue;
        }

        let full_cover_row =
            mask_res == MaskResult::FullCover && mask_slice.iter().all(|&mask| mask == OPA_COVER);

        if full_cover_row {
            for x in 0..row_width {
                rast.blend_pixel(row_area.x1 + x as i32, y, dsc.color, dsc.opa);
            }
            continue;
        }

        for (i, &mask_val) in mask_slice.iter().enumerate() {
            let final_opa = if dsc.opa == OPA_COVER {
                mask_val
            } else {
                opa_mix(dsc.opa, mask_val)
            };
            rast.blend_pixel(row_area.x1 + i as i32, y, dsc.color, final_opa);
        }
    }
}

fn build_circle_mask(width: i32) -> Vec<Opa> {
    let w = width.max(0) as usize;
    if w == 0 {
        return Vec::new();
    }

    let circle_area = Area::new(0, 0, width - 1, width - 1);
    let circle_mask = RadiusMask::new(circle_area, width / 2, false);
    let mut buffer = vec![OPA_COVER; w * w];

    for row in 0..width {
        let start = (row as usize) * w;
        let end = start + w;
        let res = circle_mask.apply(&mut buffer[start..end], 0, row);
        if res == MaskResult::Transparent {
            buffer[start..end].fill(OPA_TRANSP);
        }
    }

    buffer
}

fn add_circle(
    circle_mask: &[Opa],
    width: i32,
    blend_area: &Area,
    circle_area: &Area,
    mask_buf: &mut [Opa],
) {
    if circle_mask.is_empty() {
        return;
    }

    if let Some(common) = circle_area.intersect(blend_area) {
        let stride = width as usize;
        let circle_row = (common.y1 - circle_area.y1) as usize;
        let src_row = &circle_mask[circle_row * stride..(circle_row + 1) * stride];
        let src_offset = (common.x1 - circle_area.x1) as usize;
        let dst_offset = (common.x1 - blend_area.x1) as usize;
        let count = (common.x2 - common.x1 + 1) as usize;

        for i in 0..count {
            let sum = mask_buf[dst_offset + i] as u16 + src_row[src_offset + i] as u16;
            mask_buf[dst_offset + i] = sum.min(255) as Opa;
        }
    }
}

fn get_rounded_area(angle: i32, radius: i32, thickness: i32) -> Area {
    let thick_half = thickness / 2;
    let thick_corr = if thickness & 0x01 == 0 { 1 } else { 0 };

    let mut cir_x = ((radius - thick_half) * trigo_cos(angle)) >> (LV_TRIGO_SHIFT - 8);
    let mut cir_y = ((radius - thick_half) * trigo_sin(angle)) >> (LV_TRIGO_SHIFT - 8);

    let (x1, x2) = if cir_x > 0 {
        cir_x = (cir_x - 128) >> 8;
        (cir_x - thick_half + thick_corr, cir_x + thick_half)
    } else {
        cir_x = (cir_x + 128) >> 8;
        (cir_x - thick_half, cir_x + thick_half - thick_corr)
    };

    let (y1, y2) = if cir_y > 0 {
        cir_y = (cir_y - 128) >> 8;
        (cir_y - thick_half + thick_corr, cir_y + thick_half)
    } else {
        cir_y = (cir_y + 128) >> 8;
        (cir_y - thick_half, cir_y + thick_half - thick_corr)
    };

    Area::new(x1, y1, x2, y2)
}
