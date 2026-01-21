extern crate alloc;

use alloc::vec::Vec;

use crate::masks::RadiusMask;
use crate::primitives::gradient::gradient_get_color;
use crate::types::{Area, GradDir, Opa, OPA_COVER};
use crate::Rasterizer;

use crate::primitives::common::{clip_to_raster, effective_radius};

use super::descriptor::RectDsc;

/// Compute the background fill for the rectangle, falling back to the full
/// area when borders do not cover the edges fully.
pub fn background_fill_area(area: &Area, dsc: &RectDsc) -> Area {
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

/// Fill the background, honouring gradients and corner radii.
pub fn draw_background<R: Rasterizer>(
    rast: &mut R,
    dsc: &RectDsc,
    area: &Area,
    clip: Option<Area>,
) {
    if area.width() <= 0 || area.height() <= 0 {
        return;
    }

    let Some(mut clipped) = clip_to_raster(area, rast) else {
        return;
    };

    if let Some(clip_area) = clip {
        if let Some(intersection) = clipped.intersect(&clip_area) {
            clipped = intersection;
        } else {
            return;
        }
    }

    let mut radius = effective_radius(area, dsc.radius).max(0);
    let has_radius = radius > 0;
    let has_grad = dsc.bg_grad.dir != GradDir::None;

    let mut mask = has_radius.then(|| RadiusMask::new(*area, radius, false));
    let mut mask_buf = has_radius.then(|| Vec::with_capacity(clipped.width() as usize));

    for y in clipped.y1..=clipped.y2 {
        if let (Some(mask_obj), Some(buf)) = (&mask, &mut mask_buf) {
            buf.resize(clipped.width() as usize, 255);
            buf.fill(255);
            let _ = mask_obj.apply(buf, clipped.x1, y);
        }

        for (idx, x) in (clipped.x1..=clipped.x2).enumerate() {
            let rel_x = x - area.x1;
            let rel_y = y - area.y1;
            let (color, grad_opa) = if has_grad {
                gradient_get_color(
                    &dsc.bg_grad,
                    rel_x,
                    rel_y,
                    area.width(),
                    area.height(),
                    area.width() / 2,
                    area.height() / 2,
                )
            } else {
                (dsc.bg_color, OPA_COVER)
            };

            let mut cover = (dsc.bg_opa as u32 * grad_opa as u32) / 255;
            if let Some(buf) = &mask_buf {
                cover = (cover * buf[idx] as u32) / 255;
            }

            let cover = cover as Opa;
            if cover == 0 {
                rast.stamp_rgb_zero_alpha(x, y, color);
            } else {
                rast.blend_pixel(x, y, color, cover);
            }
        }
    }
}
