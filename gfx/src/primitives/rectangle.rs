// Ported from LVGL's software renderer (lv_draw_sw_fill/border/box_shadow).
extern crate alloc;

use crate::colors::Color;
use crate::masks::RadiusMask;
use crate::math::{dist_sq, isqrt};
use crate::primitives::gradient::gradient_get_color;
use crate::types::*;
use crate::Rasterizer;

/// Rectangle descriptor mirroring lv_draw_rect_dsc_t (subset needed by gfx).
#[derive(Clone, Debug)]
pub struct RectDsc {
    pub bg_color: Color,
    pub bg_opa: Opa,
    pub bg_grad: Gradient,
    pub radius: i32,

    pub border_color: Color,
    pub border_opa: Opa,
    pub border_width: i32,
    pub border_side: BorderSide,

    pub shadow_color: Color,
    pub shadow_opa: Opa,
    pub shadow_width: i32,
    pub shadow_offset_x: i32,
    pub shadow_offset_y: i32,
    pub shadow_spread: i32,

    pub outline_color: Color,
    pub outline_opa: Opa,
    pub outline_width: i32,
    pub outline_pad: i32,
}

impl RectDsc {
    pub fn new() -> Self {
        Self {
            bg_color: Color::WHITE,
            bg_opa: OPA_COVER,
            bg_grad: Gradient::none(),
            radius: 0,

            border_color: Color::BLACK,
            border_opa: 0,
            border_width: 0,
            border_side: BorderSide::FULL,

            shadow_color: Color::BLACK,
            shadow_opa: 0,
            shadow_width: 0,
            shadow_offset_x: 0,
            shadow_offset_y: 0,
            shadow_spread: 0,

            outline_color: Color::BLACK,
            outline_opa: 0,
            outline_width: 0,
            outline_pad: 0,
        }
    }
}

impl Default for RectDsc {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level rectangle draw matching LVGL's order: shadow → outline → fill → border.
pub fn draw_rect<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, coords: &Area) {
    if coords.width() <= 0 || coords.height() <= 0 {
        return;
    }

    let mut dirty = *coords;

    if dsc.shadow_opa > 0 && dsc.shadow_width > 0 {
        let shadow_bounds = shadow_bounds(coords, dsc);
        dirty = merge_bounds(dirty, shadow_bounds);
        render_shadow(rast, dsc, coords);
    }

    if dsc.bg_opa > 0 {
        let bg_area = background_area(coords, dsc);
        render_background(rast, dsc, &bg_area);
    }

    if dsc.border_opa > 0 && dsc.border_width > 0 && dsc.border_side != BorderSide::NONE {
        render_border(rast, dsc, coords);
    }

    if dsc.outline_opa > 0 && dsc.outline_width > 0 {
        let outline_outer = outline_outer_area(coords, dsc);
        dirty = merge_bounds(dirty, outline_outer);
        render_outline(rast, dsc, coords);
    }

    rast.mark_dirty(dirty.x1, dirty.y1, dirty.x2 + 1, dirty.y2 + 1);
}

fn render_background<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    if area.width() <= 0 || area.height() <= 0 {
        return;
    }
    if rast.width() == 0 || rast.height() == 0 {
        return;
    }

    let raster_bounds = Area::new(0, 0, rast.width() as i32 - 1, rast.height() as i32 - 1);
    let Some(clipped) = area.intersect(&raster_bounds) else {
        return;
    };

    let mut radius = effective_radius(area, dsc.radius);
    if radius < 0 {
        radius = 0;
    }
    let has_radius = radius > 0;
    let has_grad = dsc.bg_grad.dir != GradDir::None;

    if !has_radius && !has_grad && dsc.bg_opa == OPA_COVER {
        rast.fill_rect(
            clipped.x1,
            clipped.y1,
            clipped.width(),
            clipped.height(),
            dsc.bg_color,
        );
        return;
    }

    let mut mask = has_radius.then(|| RadiusMask::new(*area, radius, false));
    let mut mask_buf = has_radius.then(|| alloc::vec![255u8; clipped.width() as usize]);

    let span_width = clipped.width() as usize;
    let mut coverage_row = alloc::vec![0u8; span_width];
    let mut color_row = has_grad.then(|| alloc::vec![Color::TRANSPARENT; span_width]);

    for y in clipped.y1..=clipped.y2 {
        coverage_row.fill(0);
        if let (Some(mask_obj), Some(buf)) = (&mask, &mut mask_buf) {
            buf.fill(255);
            let _ = mask_obj.apply(buf, clipped.x1, y);
        }

        let mut any_coverage = false;
        let mut all_full = true;

        for (idx, x) in (clipped.x1..=clipped.x2).enumerate() {
            let rel_x = x - area.x1;
            let rel_y = y - area.y1;
            let (color, grad_opa) = if let Some(ref mut colors) = color_row {
                let (col, opa) = gradient_get_color(
                    &dsc.bg_grad,
                    rel_x,
                    rel_y,
                    area.width(),
                    area.height(),
                    area.width() / 2,
                    area.height() / 2,
                );
                colors[idx] = col;
                (col, opa)
            } else {
                (dsc.bg_color, OPA_COVER)
            };

            let mut cover = (dsc.bg_opa as u32 * grad_opa as u32) / 255;
            if let Some(ref buf) = mask_buf {
                cover = (cover * buf[idx] as u32) / 255;
            }
            let cover = cover as Opa;

            if cover == 0 {
                rast.stamp_rgb_zero_alpha(x, y, color);
                all_full = false;
                coverage_row[idx] = 0;
                continue;
            }

            any_coverage = true;
            if cover != OPA_COVER {
                all_full = false;
            }
            coverage_row[idx] = cover;
        }

        if !any_coverage {
            continue;
        }

        if let Some(ref colors) = color_row {
            if all_full {
                rast.blend_hspan(clipped.x1, y, colors, None);
            } else {
                rast.blend_hspan(clipped.x1, y, colors, Some(&coverage_row));
            }
        } else {
            if all_full {
                rast.fill_rect(clipped.x1, y, clipped.width(), 1, dsc.bg_color);
            } else {
                rast.blend_solid_hspan(clipped.x1, y, dsc.bg_color, &coverage_row);
            }
        }
    }
}

fn render_border<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    render_border_for_style(
        rast,
        area,
        dsc.radius,
        dsc.border_width,
        dsc.border_side,
        dsc.border_color,
        dsc.border_opa,
    );
}

fn render_outline<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    if dsc.outline_opa == 0 || dsc.outline_width <= 0 {
        return;
    }
    let ext = dsc.outline_pad + dsc.outline_width;
    let outer = Area::new(area.x1 - ext, area.y1 - ext, area.x2 + ext, area.y2 + ext);
    let raw_radius = if dsc.radius == RADIUS_CIRCLE {
        RADIUS_CIRCLE
    } else {
        dsc.radius + ext
    };
    render_border_for_style(
        rast,
        &outer,
        raw_radius,
        dsc.outline_width,
        BorderSide::FULL,
        dsc.outline_color,
        dsc.outline_opa,
    );
}

fn render_border_for_style<R: Rasterizer>(
    rast: &mut R,
    outer: &Area,
    raw_radius: i32,
    width: i32,
    sides: BorderSide,
    color: Color,
    opa: Opa,
) {
    if width <= 0 || opa == 0 || sides == BorderSide::NONE {
        return;
    }
    if outer.width() <= 0 || outer.height() <= 0 {
        return;
    }

    let short_side = outer.width().min(outer.height());
    if short_side <= 0 {
        return;
    }

    let mut rout = if raw_radius == RADIUS_CIRCLE {
        short_side / 2
    } else {
        raw_radius.min(short_side / 2).max(0)
    };
    if rout < 0 {
        rout = 0;
    }

    let bw = width.max(0);
    let inner = Area::new(
        outer.x1 + if sides.has_left() { bw } else { -(bw + rout) },
        outer.y1 + if sides.has_top() { bw } else { -(bw + rout) },
        outer.x2 - if sides.has_right() { bw } else { -(bw + rout) },
        outer.y2 - if sides.has_bottom() { bw } else { -(bw + rout) },
    );
    let rin = (rout - bw).max(0);

    if rout == 0 && rin == 0 {
        render_border_simple(rast, &inner, outer, sides, color, opa);
    } else {
        render_border_complex(rast, outer, &inner, rout, rin, sides, color, opa);
    }
}

fn render_border_simple<R: Rasterizer>(
    rast: &mut R,
    inner: &Area,
    outer: &Area,
    sides: BorderSide,
    color: Color,
    opa: Opa,
) {
    let top_side = outer.y1 <= inner.y1;
    let bottom_side = outer.y2 >= inner.y2;
    let left_side = outer.x1 <= inner.x1;
    let right_side = outer.x2 >= inner.x2;

    if top_side && sides.has_top() {
        let top = Area::new(outer.x1, outer.y1, outer.x2, inner.y1 - 1);
        fill_rect_clipped(rast, &top, color, opa);
    }

    if bottom_side && sides.has_bottom() {
        let bottom = Area::new(outer.x1, inner.y2 + 1, outer.x2, outer.y2);
        fill_rect_clipped(rast, &bottom, color, opa);
    }

    if left_side && sides.has_left() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        let left = Area::new(outer.x1, y_start, inner.x1 - 1, y_end);
        fill_rect_clipped(rast, &left, color, opa);
    }

    if right_side && sides.has_right() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        let right = Area::new(inner.x2 + 1, y_start, outer.x2, y_end);
        fill_rect_clipped(rast, &right, color, opa);
    }
}

fn render_border_complex<R: Rasterizer>(
    rast: &mut R,
    outer: &Area,
    inner: &Area,
    rout: i32,
    rin: i32,
    _sides: BorderSide,
    color: Color,
    opa: Opa,
) {
    const SPLIT_LIMIT: i32 = 50;

    if rast.width() == 0 || rast.height() == 0 {
        return;
    }

    let raster_bounds = Area::new(0, 0, rast.width() as i32 - 1, rast.height() as i32 - 1);
    let Some(draw_area) = outer.intersect(&raster_bounds) else {
        return;
    };
    if draw_area.width() <= 0 || draw_area.height() <= 0 {
        return;
    }

    let inner_mask = RadiusMask::new(*inner, rin, true);
    let outer_mask = (rout > 0).then(|| RadiusMask::new(*outer, rout, false));

    let mut core = Area::new(0, 0, -1, -1);
    core.x1 = (outer.x1 + rout).max(inner.x1);
    core.x2 = (outer.x2 - rout).min(inner.x2);
    core.y1 = (outer.y1 + rout).max(inner.y1);
    core.y2 = (outer.y2 - rout).min(inner.y2);

    let top_side = outer.y1 <= inner.y1;
    let bottom_side = outer.y2 >= inner.y2;
    let left_side = outer.x1 <= inner.x1;
    let right_side = outer.x2 >= inner.x2;

    if top_side && core.x1 <= core.x2 {
        let top = Area::new(core.x1, outer.y1, core.x2, inner.y1 - 1);
        fill_rect_clipped(rast, &top, color, opa);
    }

    if bottom_side && core.x1 <= core.x2 {
        let bottom = Area::new(core.x1, inner.y2 + 1, core.x2, outer.y2);
        fill_rect_clipped(rast, &bottom, color, opa);
    }

    if inner.x1 >= inner.x2 && left_side && right_side {
        let middle = Area::new(outer.x1, core.y1, outer.x2, core.y2);
        fill_rect_clipped(rast, &middle, color, opa);
    } else {
        if left_side {
            let left = Area::new(outer.x1, core.y1, inner.x1 - 1, core.y2);
            fill_rect_clipped(rast, &left, color, opa);
        }
        if right_side {
            let right = Area::new(inner.x2 + 1, core.y1, outer.x2, core.y2);
            fill_rect_clipped(rast, &right, color, opa);
        }
    }

    let draw_width = draw_area.width() as usize;
    if draw_width == 0 {
        return;
    }

    let mut mask_line = alloc::vec![255u8; draw_width];
    let mut inner_line = alloc::vec![255u8; draw_width];
    let mut outer_line = alloc::vec![255u8; draw_width];
    let mask_origin_x = draw_area.x1;

    let mut split_hor = true;
    if left_side && right_side && top_side && bottom_side && core.width() < SPLIT_LIMIT {
        split_hor = false;
    }

    if !split_hor {
        let max_h = rout.max(inner.y1 - outer.y1);
        for h in 0..=max_h {
            let top_y = outer.y1 + h;
            if top_y >= draw_area.y1 && top_y <= draw_area.y2 {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, top_y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    top_y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line,
                    draw_area.x1,
                    top_y,
                    inner,
                    Some(&inner_line),
                    Some(&outer_line),
                );
            }

            let bottom_y = outer.y2 - h;
            if bottom_y >= draw_area.y1 && bottom_y <= draw_area.y2 && bottom_y != top_y {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, bottom_y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    bottom_y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line,
                    draw_area.x1,
                    bottom_y,
                    inner,
                    Some(&inner_line),
                    Some(&outer_line),
                );
            }
        }
        return;
    }

    let left_span_end = (core.x1 - 1).min(draw_area.x2);
    if (left_side || top_side) && left_span_end >= draw_area.x1 {
        let start_y = draw_area.y1;
        let end_y = core.y1.min(draw_area.y2 + 1);
        if start_y < end_y {
            let span_x1 = draw_area.x1;
            let span_len = (left_span_end - span_x1 + 1) as usize;
            for y in start_y..end_y {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                let offset = (span_x1 - mask_origin_x) as usize;
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line[offset..offset + span_len],
                    span_x1,
                    y,
                    inner,
                    Some(&inner_line[offset..offset + span_len]),
                    Some(&outer_line[offset..offset + span_len]),
                );
            }
        }
    }

    if (left_side || bottom_side) && left_span_end >= draw_area.x1 {
        let start_y = (core.y2 + 1).max(draw_area.y1);
        let end_y = draw_area.y2;
        if start_y <= end_y {
            let span_x1 = draw_area.x1;
            let span_len = (left_span_end - span_x1 + 1) as usize;
            for y in start_y..=end_y {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                let offset = (span_x1 - mask_origin_x) as usize;
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line[offset..offset + span_len],
                    span_x1,
                    y,
                    inner,
                    Some(&inner_line[offset..offset + span_len]),
                    Some(&outer_line[offset..offset + span_len]),
                );
            }
        }
    }

    let right_span_start = (core.x2 + 1).max(draw_area.x1);
    if (right_side || top_side) && right_span_start <= draw_area.x2 {
        let start_y = draw_area.y1;
        let end_y = core.y1.min(draw_area.y2 + 1);
        if start_y < end_y {
            let span_x1 = right_span_start;
            let span_len = (draw_area.x2 - span_x1 + 1) as usize;
            for y in start_y..end_y {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                let offset = (span_x1 - mask_origin_x) as usize;
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line[offset..offset + span_len],
                    span_x1,
                    y,
                    inner,
                    Some(&inner_line[offset..offset + span_len]),
                    Some(&outer_line[offset..offset + span_len]),
                );
            }
        }
    }

    if (right_side || bottom_side) && right_span_start <= draw_area.x2 {
        let start_y = (core.y2 + 1).max(draw_area.y1);
        let end_y = draw_area.y2;
        if start_y <= end_y {
            let span_x1 = right_span_start;
            let span_len = (draw_area.x2 - span_x1 + 1) as usize;
            for y in start_y..=end_y {
                if let Some(ref outer_m) = outer_mask {
                    outer_line.fill(255);
                    let _ = outer_m.apply(&mut outer_line, mask_origin_x, y);
                } else {
                    outer_line.fill(255);
                }
                prepare_mask_line(
                    &mut mask_line,
                    &inner_mask,
                    outer_mask.as_ref(),
                    y,
                    mask_origin_x,
                    Some(inner_line.as_mut_slice()),
                );
                let offset = (span_x1 - mask_origin_x) as usize;
                paint_masked_span(
                    rast,
                    color,
                    opa,
                    &mask_line[offset..offset + span_len],
                    span_x1,
                    y,
                    inner,
                    Some(&inner_line[offset..offset + span_len]),
                    Some(&outer_line[offset..offset + span_len]),
                );
            }
        }
    }
}

fn fill_rect_clipped<R: Rasterizer>(rast: &mut R, rect: &Area, color: Color, opa: Opa) {
    if opa == 0 {
        return;
    }
    if rect.width() <= 0 || rect.height() <= 0 {
        return;
    }

    let raster_bounds = Area::new(0, 0, rast.width() as i32 - 1, rast.height() as i32 - 1);
    let Some(clamped) = rect.intersect(&raster_bounds) else {
        return;
    };

    if clamped.width() <= 0 || clamped.height() <= 0 {
        return;
    }

    if opa == OPA_COVER {
        rast.fill_rect(
            clamped.x1,
            clamped.y1,
            clamped.width(),
            clamped.height(),
            color,
        );
        return;
    }

    for y in clamped.y1..=clamped.y2 {
        rast.blend_hspan_with(clamped.x1, y, clamped.width(), |_| (color, opa));
    }
}

fn prepare_mask_line(
    buf: &mut [Opa],
    inner: &RadiusMask,
    outer: Option<&RadiusMask>,
    y: i32,
    x_start: i32,
    inner_snapshot: Option<&mut [Opa]>,
) {
    buf.fill(255);
    let _ = inner.apply(buf, x_start, y);
    if let Some(snapshot) = inner_snapshot {
        snapshot.copy_from_slice(buf);
    }
    if let Some(mask) = outer {
        let _ = mask.apply(buf, x_start, y);
    }
}

fn paint_masked_span<R: Rasterizer>(
    rast: &mut R,
    color: Color,
    base_opa: Opa,
    mask: &[Opa],
    span_x1: i32,
    y: i32,
    inner: &Area,
    inner_snapshot: Option<&[Opa]>,
    outer_snapshot: Option<&[Opa]>,
) {
    if mask.is_empty() || base_opa == 0 {
        return;
    }

    let _ = inner;
    let _ = inner_snapshot;

    let mut coverages = alloc::vec![0u8; mask.len()];
    let mut any = false;

    for (idx, &mask_val) in mask.iter().enumerate() {
        let x = span_x1 + idx as i32;
        if mask_val == 0 {
            let outer_zero = outer_snapshot
                .and_then(|snap| snap.get(idx))
                .map(|&v| v == 0)
                .unwrap_or(false);
            if outer_zero {
                rast.stamp_rgb_zero_alpha(x, y, color);
            }
            continue;
        }

        let coverage = ((base_opa as u32 * mask_val as u32) / 255) as Opa;
        if coverage == 0 {
            continue;
        }
        coverages[idx] = coverage;
        any = true;
    }

    if any {
        rast.blend_solid_hspan(span_x1, y, color, &coverages);
    }
}

fn render_shadow<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    if dsc.shadow_width <= 0 || dsc.shadow_opa == 0 {
        return;
    }
    if rast.width() == 0 || rast.height() == 0 {
        return;
    }

    let shadow_bounds = shadow_bounds(area, dsc);
    let raster_bounds = Area::new(0, 0, rast.width() as i32 - 1, rast.height() as i32 - 1);
    let Some(clipped) = shadow_bounds.intersect(&raster_bounds) else {
        return;
    };

    let short_side = area.width().min(area.height());
    if short_side <= 0 {
        return;
    }

    let mut radius = if dsc.radius == RADIUS_CIRCLE {
        short_side / 2
    } else {
        dsc.radius.min(short_side / 2).max(0)
    };
    if radius < 0 {
        radius = 0;
    }
    let shadow_radius = radius + dsc.shadow_spread;

    let core = Area::new(
        area.x1 + dsc.shadow_offset_x - dsc.shadow_spread,
        area.y1 + dsc.shadow_offset_y - dsc.shadow_spread,
        area.x2 + dsc.shadow_offset_x + dsc.shadow_spread,
        area.y2 + dsc.shadow_offset_y + dsc.shadow_spread,
    );

    let span_width = (clipped.x2 - clipped.x1 + 1) as usize;
    if span_width == 0 {
        return;
    }

    let mut coverage_row = alloc::vec![0u8; span_width];

    for y in clipped.y1..=clipped.y2 {
        coverage_row.fill(0);
        let mut any = false;

        for (idx, x) in (clipped.x1..=clipped.x2).enumerate() {
            let shadow_cov = calculate_shadow_opa(x, y, &core, shadow_radius, dsc.shadow_width);
            if shadow_cov == 0 {
                continue;
            }
            let final_opa = ((dsc.shadow_opa as u32 * shadow_cov as u32) / 255) as Opa;
            if final_opa == 0 {
                continue;
            }
            coverage_row[idx] = final_opa;
            any = true;
        }

        if any {
            rast.blend_solid_hspan(clipped.x1, y, dsc.shadow_color, &coverage_row);
        }
    }
}

fn calculate_shadow_opa(x: i32, y: i32, rect: &Area, radius: i32, width: i32) -> Opa {
    let cx = x.max(rect.x1).min(rect.x2);
    let cy = y.max(rect.y1).min(rect.y2);

    let corner_x = if cx == rect.x1 {
        rect.x1 + radius
    } else if cx == rect.x2 {
        rect.x2 - radius
    } else {
        cx
    };

    let corner_y = if cy == rect.y1 {
        rect.y1 + radius
    } else if cy == rect.y2 {
        rect.y2 - radius
    } else {
        cy
    };

    let dist = isqrt(dist_sq(x, y, corner_x, corner_y) as u32) as i32;
    if dist >= width {
        0
    } else {
        (((width - dist) * 255) / width).max(0) as Opa
    }
}

fn background_area(area: &Area, dsc: &RectDsc) -> Area {
    if dsc.border_width > 1 && dsc.border_opa >= OPA_COVER && dsc.radius != 0 {
        Area::new(
            area.x1 + if dsc.border_side.has_left() { 1 } else { 0 },
            area.y1 + if dsc.border_side.has_top() { 1 } else { 0 },
            area.x2 - if dsc.border_side.has_right() { 1 } else { 0 },
            area.y2 - if dsc.border_side.has_bottom() { 1 } else { 0 },
        )
    } else {
        *area
    }
}

fn outline_outer_area(area: &Area, dsc: &RectDsc) -> Area {
    let ext = dsc.outline_pad + dsc.outline_width;
    Area::new(area.x1 - ext, area.y1 - ext, area.x2 + ext, area.y2 + ext)
}

fn shadow_bounds(area: &Area, dsc: &RectDsc) -> Area {
    let core = Area::new(
        area.x1 + dsc.shadow_offset_x - dsc.shadow_spread,
        area.y1 + dsc.shadow_offset_y - dsc.shadow_spread,
        area.x2 + dsc.shadow_offset_x + dsc.shadow_spread,
        area.y2 + dsc.shadow_offset_y + dsc.shadow_spread,
    );
    Area::new(
        core.x1 - dsc.shadow_width,
        core.y1 - dsc.shadow_width,
        core.x2 + dsc.shadow_width,
        core.y2 + dsc.shadow_width,
    )
}

fn merge_bounds(a: Area, b: Area) -> Area {
    Area::new(
        a.x1.min(b.x1),
        a.y1.min(b.y1),
        a.x2.max(b.x2),
        a.y2.max(b.y2),
    )
}

fn effective_radius(area: &Area, requested: i32) -> i32 {
    if requested == RADIUS_CIRCLE {
        area.width().min(area.height()) / 2
    } else {
        requested.min(area.width().min(area.height()) / 2).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_application_keeps_zero_segments() {
        let outer = Area::new(10, 10, 30, 30);
        let inner = Area::new(12, 12, 28, 28);
        let outer_mask = RadiusMask::new(outer, 5, false);
        let inner_mask = RadiusMask::new(inner, 3, true);

        let width = outer.width() as usize;
        let mut buf = alloc::vec![255u8; width];
        prepare_mask_line(&mut buf, &inner_mask, Some(&outer_mask), 12, outer.x1, None);
        assert!(buf.iter().any(|&v| v == 0));
    }
}
