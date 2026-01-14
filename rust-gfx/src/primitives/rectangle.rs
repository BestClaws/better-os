use crate::color::Rgba8888;
use crate::math::{aa_coverage_sq, dist_sq, isqrt};
use crate::primitives::{gradient::*, mask::RadiusMask};
use crate::types::*;
/// Rectangle drawing matching LVGL's lv_draw_rect functionality
/// Supports: solid fills, gradients, borders, shadows, outlines, rounded corners
use crate::Rasterizer;

extern crate alloc;

/// Rectangle descriptor matching LVGL's lv_draw_rect_dsc_t
#[derive(Clone, Debug)]
pub struct RectDsc {
    /// Background color
    pub bg_color: Rgba8888,
    /// Background opacity
    pub bg_opa: Opa,
    /// Background gradient
    pub bg_grad: Gradient,
    /// Corner radius (can be RADIUS_CIRCLE for circular)
    pub radius: i32,

    /// Border color
    pub border_color: Rgba8888,
    /// Border opacity
    pub border_opa: Opa,
    /// Border width
    pub border_width: i32,
    /// Border sides
    pub border_side: BorderSide,

    /// Shadow color
    pub shadow_color: Rgba8888,
    /// Shadow opacity
    pub shadow_opa: Opa,
    /// Shadow width (blur radius)
    pub shadow_width: i32,
    /// Shadow X offset
    pub shadow_offset_x: i32,
    /// Shadow Y offset
    pub shadow_offset_y: i32,
    /// Shadow spread
    pub shadow_spread: i32,

    /// Outline color
    pub outline_color: Rgba8888,
    /// Outline opacity
    pub outline_opa: Opa,
    /// Outline width
    pub outline_width: i32,
    /// Outline padding (distance from border)
    pub outline_pad: i32,
}

impl RectDsc {
    /// Initialize with LVGL defaults
    pub fn new() -> Self {
        Self {
            bg_color: Rgba8888::WHITE,
            bg_opa: OPA_COVER,
            bg_grad: Gradient::none(),
            radius: 0,

            border_color: Rgba8888::BLACK,
            border_opa: 0,
            border_width: 0,
            border_side: BorderSide::FULL,

            shadow_color: Rgba8888::BLACK,
            shadow_opa: 0,
            shadow_width: 0,
            shadow_offset_x: 0,
            shadow_offset_y: 0,
            shadow_spread: 0,

            outline_color: Rgba8888::BLACK,
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

/// Draw a rectangle with the given descriptor
pub fn draw_rect<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    // Calculate extended area including shadow and outline
    let mut min_x = area.x1;
    let mut min_y = area.y1;
    let mut max_x = area.x2;
    let mut max_y = area.y2;

    // Extend for shadow
    if dsc.shadow_opa > 0 && dsc.shadow_width > 0 {
        min_x = min_x.min(area.x1 + dsc.shadow_offset_x - dsc.shadow_width);
        min_y = min_y.min(area.y1 + dsc.shadow_offset_y - dsc.shadow_width);
        max_x = max_x.max(area.x2 + dsc.shadow_offset_x + dsc.shadow_width);
        max_y = max_y.max(area.y2 + dsc.shadow_offset_y + dsc.shadow_width);
    }

    // Extend for outline
    if dsc.outline_opa > 0 && dsc.outline_width > 0 {
        let outline_ext = dsc.outline_pad + dsc.outline_width;
        min_x = min_x.min(area.x1 - outline_ext);
        min_y = min_y.min(area.y1 - outline_ext);
        max_x = max_x.max(area.x2 + outline_ext);
        max_y = max_y.max(area.y2 + outline_ext);
    }

    // Draw shadow first (if any)
    if dsc.shadow_opa > 0 && dsc.shadow_width > 0 {
        draw_shadow(rast, dsc, area);
    }

    // Draw outline (if any)
    if dsc.outline_opa > 0 && dsc.outline_width > 0 {
        draw_outline(rast, dsc, area);
    }

    // Draw background
    if dsc.bg_opa > 0 {
        // LVGL optimization: If border is opaque and thick, shrink bg by 1px to avoid corner artifacts
        let bg_area = if dsc.border_width > 1 && dsc.border_opa >= OPA_COVER && dsc.radius != 0 {
            Area::new(
                area.x1 + if dsc.border_side.has_left() { 1 } else { 0 },
                area.y1 + if dsc.border_side.has_top() { 1 } else { 0 },
                area.x2 - if dsc.border_side.has_right() { 1 } else { 0 },
                area.y2 - if dsc.border_side.has_bottom() { 1 } else { 0 },
            )
        } else {
            *area
        };
        draw_bg(rast, dsc, &bg_area);
    }

    // Draw border (if any)
    if dsc.border_opa > 0 && dsc.border_width > 0 {
        draw_border(rast, dsc, area);
    }

    // Mark the entire affected area as dirty
    rast.mark_dirty(min_x, min_y, max_x + 1, max_y + 1);
}

/// Draw rectangle background (with gradient support and rounded corners)
fn draw_bg<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    let width = area.width();
    let height = area.height();

    if width <= 0 || height <= 0 {
        return;
    }

    // Calculate actual radius
    let short_side = width.min(height);
    let mut radius = dsc.radius.min(short_side / 2);

    // Handle LV_RADIUS_CIRCLE
    if dsc.radius == RADIUS_CIRCLE {
        radius = short_side / 2;
    }

    let has_radius = radius > 0;
    let has_grad = dsc.bg_grad.dir != GradDir::None;

    // Get center for gradients
    let cx = area.x1 + width / 2;
    let cy = area.y1 + height / 2;

    // Simple case: no radius, no gradient
    if !has_radius && !has_grad {
        if dsc.bg_opa == OPA_COVER {
            rast.fill_rect(area.x1, area.y1, width, height, dsc.bg_color);
        } else {
            // Need to blend with opacity
            for y in area.y1..=area.y2 {
                for x in area.x1..=area.x2 {
                    rast.blend_pixel(x, y, dsc.bg_color, dsc.bg_opa);
                }
            }
        }
        return;
    }

    // Complex case: radius and/or gradient
    let mask = if has_radius {
        Some(RadiusMask::new(*area, radius, false))
    } else {
        None
    };

    for y in area.y1..=area.y2 {
        for x in area.x1..=area.x2 {
            // Get mask value for rounded corners
            let mask_val = if let Some(ref m) = mask {
                m.get_mask_value(x, y)
            } else {
                OPA_COVER
            };

            // Get color (possibly from gradient)
            let (color, grad_opa) = if has_grad {
                let rel_x = x - area.x1;
                let rel_y = y - area.y1;
                gradient_get_color(
                    &dsc.bg_grad,
                    rel_x,
                    rel_y,
                    width,
                    height,
                    width / 2,
                    height / 2,
                )
            } else {
                (dsc.bg_color, OPA_COVER)
            };

            // Combine opacities: dsc.bg_opa * grad_opa * mask_val
            let opa =
                ((dsc.bg_opa as u32 * grad_opa as u32 * mask_val as u32) / (255 * 255)) as Opa;

            if opa == 0 {
                // Match LVGL: retain RGB even when fully masked out
                rast.stamp_rgb_zero_alpha(x, y, color);
            } else {
                rast.blend_pixel(x, y, color, opa);
            }
        }
    }
}

/// Draw rectangle border matching LVGL exactly
fn draw_border<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    let width = area.width();
    let height = area.height();

    if width <= 0 || height <= 0 {
        return;
    }

    let short_side = width.min(height);
    let mut rout = dsc.radius.min(short_side / 2);

    if dsc.radius == RADIUS_CIRCLE {
        rout = short_side / 2;
    }

    let bw = dsc.border_width;
    let sides = dsc.border_side;

    // Calculate inner area (LVGL logic)
    let inner_area = Area::new(
        area.x1 + if sides.has_left() { bw } else { -(bw + rout) },
        area.y1 + if sides.has_top() { bw } else { -(bw + rout) },
        area.x2 - if sides.has_right() { bw } else { -(bw + rout) },
        area.y2 - if sides.has_bottom() { bw } else { -(bw + rout) },
    );
    let rin = (rout - bw).max(0);

    // If no radius, use simple border
    if rout == 0 && rin == 0 {
        draw_border_simple(rast, dsc, area, &inner_area);
        return;
    }

    // Complex border with radius
    draw_border_complex(rast, dsc, area, &inner_area, rout, rin);
}

/// Simple border without rounded corners (matches LVGL exactly)
fn draw_border_simple<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, outer: &Area, inner: &Area) {
    let sides = dsc.border_side;
    let top_side = outer.y1 <= inner.y1;
    let bottom_side = outer.y2 >= inner.y2;
    let left_side = outer.x1 <= inner.x1;
    let right_side = outer.x2 >= inner.x2;

    // Top edge
    if top_side && sides.has_top() {
        for y in outer.y1..inner.y1 {
            for x in outer.x1..=outer.x2 {
                rast.blend_pixel(x, y, dsc.border_color, dsc.border_opa);
            }
        }
    }

    // Bottom edge
    if bottom_side && sides.has_bottom() {
        for y in (inner.y2 + 1)..=outer.y2 {
            for x in outer.x1..=outer.x2 {
                rast.blend_pixel(x, y, dsc.border_color, dsc.border_opa);
            }
        }
    }

    // Left edge - adjust Y range based on top/bottom sides (LVGL behavior)
    if left_side && sides.has_left() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        for y in y_start..=y_end {
            for x in outer.x1..inner.x1 {
                rast.blend_pixel(x, y, dsc.border_color, dsc.border_opa);
            }
        }
    }

    // Right edge - adjust Y range based on top/bottom sides (LVGL behavior)
    if right_side && sides.has_right() {
        let y_start = if top_side { inner.y1 } else { outer.y1 };
        let y_end = if bottom_side { inner.y2 } else { outer.y2 };
        for y in y_start..=y_end {
            for x in (inner.x2 + 1)..=outer.x2 {
                rast.blend_pixel(x, y, dsc.border_color, dsc.border_opa);
            }
        }
    }
}

/// Complex border with rounded corners (matches LVGL exactly - scanline approach)
fn draw_border_complex<R: Rasterizer>(
    rast: &mut R,
    dsc: &RectDsc,
    outer: &Area,
    inner: &Area,
    rout: i32,
    rin: i32,
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
    let outer_mask = if rout > 0 {
        Some(RadiusMask::new(*outer, rout, false))
    } else {
        None
    };
    let outer_mask_ref = outer_mask.as_ref();

    let mut core_area = Area::new(0, 0, -1, -1);
    core_area.x1 = (outer.x1 + rout).max(inner.x1);
    core_area.x2 = (outer.x2 - rout).min(inner.x2);
    core_area.y1 = (outer.y1 + rout).max(inner.y1);
    core_area.y2 = (outer.y2 - rout).min(inner.y2);

    let top_side = outer.y1 <= inner.y1;
    let bottom_side = outer.y2 >= inner.y2;
    let left_side = outer.x1 <= inner.x1;
    let right_side = outer.x2 >= inner.x2;

    let core_w = core_area.width();

    let mut split_hor = true;
    if left_side && right_side && top_side && bottom_side && core_w < SPLIT_LIMIT {
        split_hor = false;
    }

    if top_side && core_area.x1 <= core_area.x2 {
        let top_area = Area::new(core_area.x1, outer.y1, core_area.x2, inner.y1 - 1);
        blend_rect_clipped(
            rast,
            &top_area,
            &draw_area,
            dsc.border_color,
            dsc.border_opa,
        );
    }

    if bottom_side && core_area.x1 <= core_area.x2 {
        let bottom_area = Area::new(core_area.x1, inner.y2 + 1, core_area.x2, outer.y2);
        blend_rect_clipped(
            rast,
            &bottom_area,
            &draw_area,
            dsc.border_color,
            dsc.border_opa,
        );
    }

    if inner.x1 >= inner.x2 && left_side && right_side {
        let middle_area = Area::new(outer.x1, core_area.y1, outer.x2, core_area.y2);
        blend_rect_clipped(
            rast,
            &middle_area,
            &draw_area,
            dsc.border_color,
            dsc.border_opa,
        );
    } else {
        if left_side && core_area.y1 <= core_area.y2 {
            let left_area = Area::new(outer.x1, core_area.y1, inner.x1 - 1, core_area.y2);
            blend_rect_clipped(
                rast,
                &left_area,
                &draw_area,
                dsc.border_color,
                dsc.border_opa,
            );
        }

        if right_side && core_area.y1 <= core_area.y2 {
            let right_area = Area::new(inner.x2 + 1, core_area.y1, outer.x2, core_area.y2);
            blend_rect_clipped(
                rast,
                &right_area,
                &draw_area,
                dsc.border_color,
                dsc.border_opa,
            );
        }
    }

    let draw_width = draw_area.width() as usize;
    if draw_width == 0 {
        return;
    }
    let mut mask_buf: alloc::vec::Vec<Opa> = alloc::vec![255; draw_width];
    let mask_origin_x = draw_area.x1;

    if !split_hor {
        let max_h = rout.max(inner.y1 - outer.y1);
        for h in 0..=max_h {
            let top_y = outer.y1 + h;
            if top_y >= draw_area.y1 && top_y <= draw_area.y2 {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    top_y,
                    mask_origin_x,
                );
                draw_masked_span(rast, dsc, mask_buf.as_slice(), draw_area.x1, top_y);
            }

            let bottom_y = outer.y2 - h;
            if bottom_y >= draw_area.y1 && bottom_y <= draw_area.y2 && bottom_y != top_y {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    bottom_y,
                    mask_origin_x,
                );
                draw_masked_span(rast, dsc, mask_buf.as_slice(), draw_area.x1, bottom_y);
            }
        }
        return;
    }

    let left_span_end = (core_area.x1 - 1).min(draw_area.x2);
    if (left_side || top_side) && left_span_end >= draw_area.x1 {
        let start_y = draw_area.y1;
        let end_y = core_area.y1.min(draw_area.y2 + 1);
        if start_y < end_y {
            let span_x1 = draw_area.x1;
            let span_len = (left_span_end - span_x1 + 1) as usize;
            for y in start_y..end_y {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    y,
                    mask_origin_x,
                );
                let start_idx = (span_x1 - mask_origin_x) as usize;
                let end_idx = start_idx + span_len;
                draw_masked_span(rast, dsc, &mask_buf[start_idx..end_idx], span_x1, y);
            }
        }
    }

    if (left_side || bottom_side) && left_span_end >= draw_area.x1 {
        let start_y = (core_area.y2 + 1).max(draw_area.y1);
        let end_y = draw_area.y2;
        if start_y <= end_y {
            let span_x1 = draw_area.x1;
            let span_len = (left_span_end - span_x1 + 1) as usize;
            for y in start_y..=end_y {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    y,
                    mask_origin_x,
                );
                let start_idx = (span_x1 - mask_origin_x) as usize;
                let end_idx = start_idx + span_len;
                draw_masked_span(rast, dsc, &mask_buf[start_idx..end_idx], span_x1, y);
            }
        }
    }

    let right_span_start = (core_area.x2 + 1).max(draw_area.x1);
    if (right_side || top_side) && right_span_start <= draw_area.x2 {
        let start_y = draw_area.y1;
        let end_y = core_area.y1.min(draw_area.y2 + 1);
        if start_y < end_y {
            let span_x1 = right_span_start;
            let span_len = (draw_area.x2 - span_x1 + 1) as usize;
            for y in start_y..end_y {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    y,
                    mask_origin_x,
                );
                let start_idx = (span_x1 - mask_origin_x) as usize;
                let end_idx = start_idx + span_len;
                draw_masked_span(rast, dsc, &mask_buf[start_idx..end_idx], span_x1, y);
            }
        }
    }

    if (right_side || bottom_side) && right_span_start <= draw_area.x2 {
        let start_y = (core_area.y2 + 1).max(draw_area.y1);
        let end_y = draw_area.y2;
        if start_y <= end_y {
            let span_x1 = right_span_start;
            let span_len = (draw_area.x2 - span_x1 + 1) as usize;
            for y in start_y..=end_y {
                prepare_mask_line(
                    mask_buf.as_mut_slice(),
                    &inner_mask,
                    outer_mask_ref,
                    y,
                    mask_origin_x,
                );
                let start_idx = (span_x1 - mask_origin_x) as usize;
                let end_idx = start_idx + span_len;
                draw_masked_span(rast, dsc, &mask_buf[start_idx..end_idx], span_x1, y);
            }
        }
    }
}

fn prepare_mask_line(
    mask_buf: &mut [Opa],
    inner_mask: &RadiusMask,
    outer_mask: Option<&RadiusMask>,
    y: i32,
    x_start: i32,
) {
    for m in mask_buf.iter_mut() {
        *m = 255;
    }
    inner_mask.apply_to_line(y, x_start, mask_buf);
    if let Some(mask) = outer_mask {
        mask.apply_to_line(y, x_start, mask_buf);
    }
}

fn blend_rect_clipped<R: Rasterizer>(
    rast: &mut R,
    area: &Area,
    clip: &Area,
    color: Rgba8888,
    opa: Opa,
) {
    if opa == 0 {
        return;
    }
    if let Some(clamped) = area.intersect(clip) {
        if clamped.width() <= 0 || clamped.height() <= 0 {
            return;
        }
        for y in clamped.y1..=clamped.y2 {
            for x in clamped.x1..=clamped.x2 {
                rast.blend_pixel(x, y, color, opa);
            }
        }
    }
}

fn draw_masked_span<R: Rasterizer>(
    rast: &mut R,
    dsc: &RectDsc,
    mask_buf: &[Opa],
    span_x1: i32,
    y: i32,
) {
    if mask_buf.is_empty() || dsc.border_opa == 0 {
        return;
    }
    let len = mask_buf.len();
    let mut idx = 0;
    while idx < len {
        let mask_val = mask_buf[idx];
        if mask_val == 0 {
            let mut run_end = idx + 1;
            while run_end < len && mask_buf[run_end] == 0 {
                run_end += 1;
            }
            let left = if idx > 0 { mask_buf[idx - 1] } else { 0 };
            let right = if run_end < len { mask_buf[run_end] } else { 0 };
            let left_nonzero = left > 0;
            let right_nonzero = right > 0;
            if left_nonzero ^ right_nonzero {
                for offset in idx..run_end {
                    let x = span_x1 + offset as i32;
                    rast.stamp_rgb_zero_alpha(x, y, dsc.border_color);
                }
            }
            idx = run_end;
            continue;
        }

        let product = dsc.border_opa as u32 * mask_val as u32;
        let opa = ((product * 0x8081) >> 23) as Opa;
        if opa == 0 {
            // Handle extremely small coverage by preserving border RGB on both sides
            let left = if idx > 0 { mask_buf[idx - 1] } else { 0 };
            let right = if idx + 1 < len { mask_buf[idx + 1] } else { 0 };
            let left_nonzero = left > 0;
            let right_nonzero = right > 0;
            if left_nonzero ^ right_nonzero {
                let x = span_x1 + idx as i32;
                rast.stamp_rgb_zero_alpha(x, y, dsc.border_color);
            }
            idx += 1;
            continue;
        }

        let x = span_x1 + idx as i32;
        rast.blend_pixel(x, y, dsc.border_color, opa);
        idx += 1;
    }
}

/// Draw rectangle shadow
fn draw_shadow<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    let shadow_area = Area::new(
        area.x1 + dsc.shadow_offset_x - dsc.shadow_width,
        area.y1 + dsc.shadow_offset_y - dsc.shadow_width,
        area.x2 + dsc.shadow_offset_x + dsc.shadow_width,
        area.y2 + dsc.shadow_offset_y + dsc.shadow_width,
    );

    let width = area.width();
    let height = area.height();
    let short_side = width.min(height);
    let mut radius = dsc.radius.min(short_side / 2);

    if dsc.radius == RADIUS_CIRCLE {
        radius = short_side / 2;
    }

    // Adjust for spread
    let shadow_radius = radius + dsc.shadow_spread;

    for y in shadow_area.y1..=shadow_area.y2 {
        for x in shadow_area.x1..=shadow_area.x2 {
            // Calculate distance from rect edge
            let shadow_opa = calculate_shadow_opa(x, y, area, shadow_radius, dsc.shadow_width);

            // Draw even if shadow_opa == 0 to preserve color info (non-premultiplied alpha)
            let final_opa = ((dsc.shadow_opa as u32 * shadow_opa as u32) / 255) as Opa;
            rast.blend_pixel(x, y, dsc.shadow_color, final_opa);
        }
    }
}

/// Calculate shadow opacity at a point
fn calculate_shadow_opa(x: i32, y: i32, rect: &Area, radius: i32, shadow_width: i32) -> Opa {
    // Find closest point on rectangle
    let cx = x.max(rect.x1).min(rect.x2);
    let cy = y.max(rect.y1).min(rect.y2);

    // Check if we need to account for corners
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

    let dist_sq = dist_sq(x, y, corner_x, corner_y);
    let dist = isqrt(dist_sq as u32) as i32;

    if dist >= shadow_width {
        0
    } else {
        // Linear falloff
        ((shadow_width - dist) * 255 / shadow_width) as Opa
    }
}

/// Draw rectangle outline
fn draw_outline<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    let outline_area = Area::new(
        area.x1 - dsc.outline_pad - dsc.outline_width,
        area.y1 - dsc.outline_pad - dsc.outline_width,
        area.x2 + dsc.outline_pad + dsc.outline_width,
        area.y2 + dsc.outline_pad + dsc.outline_width,
    );

    let inner_area = Area::new(
        area.x1 - dsc.outline_pad,
        area.y1 - dsc.outline_pad,
        area.x2 + dsc.outline_pad,
        area.y2 + dsc.outline_pad,
    );

    let width = area.width();
    let height = area.height();
    let short_side = width.min(height);
    let mut radius = dsc.radius.min(short_side / 2);

    if dsc.radius == RADIUS_CIRCLE {
        radius = short_side / 2;
    }

    let outline_radius = radius + dsc.outline_pad;

    for y in outline_area.y1..=outline_area.y2 {
        for x in outline_area.x1..=outline_area.x2 {
            // Check if in outline ring
            let in_outer =
                is_point_in_rounded_rect(x, y, &outline_area, outline_radius + dsc.outline_width);
            let in_inner = is_point_in_rounded_rect(x, y, &inner_area, outline_radius);

            // Draw with appropriate opacity (0 if outside outline ring)
            let opa = if in_outer && !in_inner {
                dsc.outline_opa
            } else {
                0
            };
            rast.blend_pixel(x, y, dsc.outline_color, opa);
        }
    }
}

/// Check if point is inside a rounded rectangle
fn is_point_in_rounded_rect(x: i32, y: i32, rect: &Area, radius: i32) -> bool {
    if x < rect.x1 || x > rect.x2 || y < rect.y1 || y > rect.y2 {
        return false;
    }

    if radius == 0 {
        return true;
    }

    // Check if in corner region
    let tl_x = rect.x1 + radius;
    let tl_y = rect.y1 + radius;
    let br_x = rect.x2 - radius;
    let br_y = rect.y2 - radius;

    // In corner regions, check distance
    if x < tl_x && y < tl_y {
        dist_sq(x, y, tl_x, tl_y) <= radius * radius
    } else if x > br_x && y < tl_y {
        dist_sq(x, y, br_x, tl_y) <= radius * radius
    } else if x < tl_x && y > br_y {
        dist_sq(x, y, tl_x, br_y) <= radius * radius
    } else if x > br_x && y > br_y {
        dist_sq(x, y, br_x, br_y) <= radius * radius
    } else {
        true
    }
}
