extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::color::Rgba8888;
use crate::masks::RadiusMask;
use crate::types::{Area, BorderSide, Opa};
use crate::Rasterizer;

use crate::primitives::common::{clip_to_raster, effective_radius, fill_rect_with_clipping};

use super::descriptor::RectDsc;

/// Draw the border portion of a rectangle using the supplied descriptor.
pub fn draw_border<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    if dsc.border_opa == 0 || dsc.border_width <= 0 || dsc.border_side == BorderSide::NONE {
        return;
    }

    draw_stroke(
        rast,
        area,
        dsc.radius,
        dsc.border_width,
        dsc.border_side,
        dsc.border_color,
        dsc.border_opa,
    );
}

/// Draw a stroked rounded rectangle.
pub fn draw_stroke<R: Rasterizer>(
    rast: &mut R,
    outer: &Area,
    raw_radius: i32,
    width: i32,
    sides: BorderSide,
    color: Rgba8888,
    opa: Opa,
) {
    if width <= 0 || opa == 0 || sides == BorderSide::NONE {
        return;
    }
    if outer.width() <= 0 || outer.height() <= 0 {
        return;
    }

    let mut rout = effective_radius(outer, raw_radius).max(0);
    let bw = width.max(0);

    let inner = Area::new(
        outer.x1 + if sides.has_left() { bw } else { -(bw + rout) },
        outer.y1 + if sides.has_top() { bw } else { -(bw + rout) },
        outer.x2 - if sides.has_right() { bw } else { -(bw + rout) },
        outer.y2 - if sides.has_bottom() { bw } else { -(bw + rout) },
    );
    let rin = (rout - bw).max(0);

    if rout == 0 && rin == 0 {
        draw_simple_stroke(rast, &inner, outer, sides, color, opa);
    } else {
        draw_rounded_stroke(rast, outer, &inner, rout, rin, sides, color, opa);
    }
}

fn draw_simple_stroke<R: Rasterizer>(
    rast: &mut R,
    inner: &Area,
    outer: &Area,
    sides: BorderSide,
    color: Rgba8888,
    opa: Opa,
) {
    let top_side = outer.y1 <= inner.y1;
    let bottom_side = outer.y2 >= inner.y2;
    let left_side = outer.x1 <= inner.x1;
    let right_side = outer.x2 >= inner.x2;

    if top_side && sides.has_top() {
        let top = Area::new(outer.x1, outer.y1, outer.x2, inner.y1 - 1);
        fill_rect_with_clipping(rast, &top, color, opa);
    }

    if bottom_side && sides.has_bottom() {
        let bottom = Area::new(outer.x1, inner.y2 + 1, outer.x2, outer.y2);
        fill_rect_with_clipping(rast, &bottom, color, opa);
    }

    if left_side && sides.has_left() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        let left = Area::new(outer.x1, y_start, inner.x1 - 1, y_end);
        fill_rect_with_clipping(rast, &left, color, opa);
    }

    if right_side && sides.has_right() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        let right = Area::new(inner.x2 + 1, y_start, outer.x2, y_end);
        fill_rect_with_clipping(rast, &right, color, opa);
    }
}

fn draw_rounded_stroke<R: Rasterizer>(
    rast: &mut R,
    outer: &Area,
    inner: &Area,
    rout: i32,
    rin: i32,
    sides: BorderSide,
    color: Rgba8888,
    opa: Opa,
) {
    const SPLIT_LIMIT: i32 = 50;

    let Some(draw_area) = clip_to_raster(outer, rast) else {
        return;
    };
    if draw_area.width() <= 0 || draw_area.height() <= 0 {
        return;
    }

    let inner_mask = RadiusMask::new(*inner, rin, true);
    let outer_mask = (rout > 0).then(|| RadiusMask::new(*outer, rout, false));
    let outer_mask_ref = outer_mask.as_ref();

    let mut core = Area::new(0, 0, -1, -1);
    core.x1 = (outer.x1 + rout).max(inner.x1);
    core.x2 = (outer.x2 - rout).min(inner.x2);
    core.y1 = (outer.y1 + rout).max(inner.y1);
    core.y2 = (outer.y2 - rout).min(inner.y2);

    let top_side = outer.y1 <= inner.y1 && sides.has_top();
    let bottom_side = outer.y2 >= inner.y2 && sides.has_bottom();
    let left_side = outer.x1 <= inner.x1 && sides.has_left();
    let right_side = outer.x2 >= inner.x2 && sides.has_right();

    if top_side && core.x1 <= core.x2 {
        let top = Area::new(core.x1, outer.y1, core.x2, inner.y1 - 1);
        fill_rect_with_clipping(rast, &top, color, opa);
    }

    if bottom_side && core.x1 <= core.x2 {
        let bottom = Area::new(core.x1, inner.y2 + 1, core.x2, outer.y2);
        fill_rect_with_clipping(rast, &bottom, color, opa);
    }

    if inner.x1 >= inner.x2 && left_side && right_side {
        let middle = Area::new(outer.x1, core.y1, outer.x2, core.y2);
        fill_rect_with_clipping(rast, &middle, color, opa);
    } else {
        if left_side {
            let left = Area::new(outer.x1, core.y1, inner.x1 - 1, core.y2);
            fill_rect_with_clipping(rast, &left, color, opa);
        }
        if right_side {
            let right = Area::new(inner.x2 + 1, core.y1, outer.x2, core.y2);
            fill_rect_with_clipping(rast, &right, color, opa);
        }
    }

    let draw_width = draw_area.width() as usize;
    if draw_width == 0 {
        return;
    }

    let mask_origin_x = draw_area.x1;
    let mut buffers = BorderMaskBuffers::new(draw_width);

    let mut split_hor = true;
    if left_side && right_side && top_side && bottom_side && core.width() < SPLIT_LIMIT {
        split_hor = false;
    }

    if !split_hor {
        let max_h = rout.max(inner.y1 - outer.y1);
        for h in 0..=max_h {
            let top_y = outer.y1 + h;
            if top_y >= draw_area.y1 && top_y <= draw_area.y2 {
                paint_full_row(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    draw_area.x1,
                    draw_width,
                    top_y,
                );
            }

            let bottom_y = outer.y2 - h;
            if bottom_y >= draw_area.y1 && bottom_y <= draw_area.y2 && bottom_y != top_y {
                paint_full_row(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    draw_area.x1,
                    draw_width,
                    bottom_y,
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
                paint_row_segment(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    span_x1,
                    span_len,
                    y,
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
                paint_row_segment(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    span_x1,
                    span_len,
                    y,
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
                paint_row_segment(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    span_x1,
                    span_len,
                    y,
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
                paint_row_segment(
                    rast,
                    &mut buffers,
                    &inner_mask,
                    outer_mask_ref,
                    mask_origin_x,
                    color,
                    opa,
                    span_x1,
                    span_len,
                    y,
                );
            }
        }
    }
}

struct BorderMaskBuffers {
    mask: Vec<Opa>,
    outer_snapshot: Vec<Opa>,
}

impl BorderMaskBuffers {
    fn new(width: usize) -> Self {
        Self {
            mask: vec![255; width],
            outer_snapshot: vec![255; width],
        }
    }

    fn prepare_row(
        &mut self,
        inner_mask: &RadiusMask,
        outer_mask: Option<&RadiusMask>,
        origin_x: i32,
        y: i32,
    ) {
        self.mask.fill(255);
        let _ = inner_mask.apply(&mut self.mask, origin_x, y);

        self.outer_snapshot.fill(255);
        if let Some(mask) = outer_mask {
            let _ = mask.apply(&mut self.outer_snapshot, origin_x, y);
            for (mask_val, outer_val) in self.mask.iter_mut().zip(&self.outer_snapshot) {
                *mask_val = (*mask_val).min(*outer_val);
            }
        }
    }

    fn span(&self, offset: usize, len: usize) -> (&[Opa], &[Opa]) {
        (
            &self.mask[offset..offset + len],
            &self.outer_snapshot[offset..offset + len],
        )
    }
}

fn paint_full_row<R: Rasterizer>(
    rast: &mut R,
    buffers: &mut BorderMaskBuffers,
    inner_mask: &RadiusMask,
    outer_mask: Option<&RadiusMask>,
    mask_origin_x: i32,
    color: Rgba8888,
    base_opa: Opa,
    span_x1: i32,
    span_len: usize,
    y: i32,
) {
    if span_len == 0 {
        return;
    }

    if span_x1 < mask_origin_x {
        return;
    }

    buffers.prepare_row(inner_mask, outer_mask, mask_origin_x, y);
    let offset = (span_x1 - mask_origin_x) as usize;
    let (mask_slice, outer_slice) = buffers.span(offset, span_len);
    blend_masked_span(
        rast,
        color,
        base_opa,
        mask_slice,
        span_x1,
        y,
        Some(outer_slice),
    );
}

fn paint_row_segment<R: Rasterizer>(
    rast: &mut R,
    buffers: &mut BorderMaskBuffers,
    inner_mask: &RadiusMask,
    outer_mask: Option<&RadiusMask>,
    mask_origin_x: i32,
    color: Rgba8888,
    base_opa: Opa,
    span_x1: i32,
    span_len: usize,
    y: i32,
) {
    if span_len == 0 {
        return;
    }

    if span_x1 < mask_origin_x {
        return;
    }

    buffers.prepare_row(inner_mask, outer_mask, mask_origin_x, y);
    let offset = (span_x1 - mask_origin_x) as usize;
    let (mask_slice, outer_slice) = buffers.span(offset, span_len);
    blend_masked_span(
        rast,
        color,
        base_opa,
        mask_slice,
        span_x1,
        y,
        Some(outer_slice),
    );
}

fn blend_masked_span<R: Rasterizer>(
    rast: &mut R,
    color: Rgba8888,
    base_opa: Opa,
    mask: &[Opa],
    span_x1: i32,
    y: i32,
    outer_snapshot: Option<&[Opa]>,
) {
    if mask.is_empty() || base_opa == 0 {
        return;
    }

    for (idx, &mask_val) in mask.iter().enumerate() {
        let x = span_x1 + idx as i32;
        if mask_val == 0 {
            let is_outer_zero = outer_snapshot
                .and_then(|snap| snap.get(idx))
                .map(|&v| v == 0)
                .unwrap_or(false);
            if is_outer_zero {
                rast.stamp_rgb_zero_alpha(x, y, color);
            }
            continue;
        }

        let coverage = ((base_opa as u32 * mask_val as u32) / 255) as Opa;
        if coverage == 0 {
            continue;
        }
        rast.blend_pixel(x, y, color, coverage);
    }
}
