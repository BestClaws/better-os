use crate::math::{dist_sq, isqrt};
use crate::primitives::common::{clip_to_raster, effective_radius};
use crate::types::{Area, Opa};
use crate::Rasterizer;

use super::descriptor::RectDsc;

/// Compute the outer bounds touched by the rectangle's shadow.
pub fn shadow_bounds(area: &Area, dsc: &RectDsc) -> Area {
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

/// Paint the drop shadow around the rectangle.
pub fn draw_shadow<R: Rasterizer>(
    rast: &mut R,
    dsc: &RectDsc,
    area: &Area,
    clip: Option<Area>,
) {
    if dsc.shadow_width <= 0 || dsc.shadow_opa == 0 {
        return;
    }

    let Some(mut clipped) = clip_to_raster(&shadow_bounds(area, dsc), rast) else {
        return;
    };

    if let Some(clip_area) = clip {
        if let Some(intersection) = clipped.intersect(&clip_area) {
            clipped = intersection;
        } else {
            return;
        }
    }

    if clipped.width() <= 0 || clipped.height() <= 0 {
        return;
    }

    let shadow_radius = effective_radius(area, dsc.radius).max(0) + dsc.shadow_spread;

    let core = Area::new(
        area.x1 + dsc.shadow_offset_x - dsc.shadow_spread,
        area.y1 + dsc.shadow_offset_y - dsc.shadow_spread,
        area.x2 + dsc.shadow_offset_x + dsc.shadow_spread,
        area.y2 + dsc.shadow_offset_y + dsc.shadow_spread,
    );

    for y in clipped.y1..=clipped.y2 {
        for x in clipped.x1..=clipped.x2 {
            let shadow_cov = shadow_coverage(x, y, &core, shadow_radius, dsc.shadow_width);
            if shadow_cov == 0 {
                continue;
            }

            let final_opa = ((dsc.shadow_opa as u32 * shadow_cov as u32) / 255) as Opa;
            if final_opa == 0 {
                continue;
            }

            rast.blend_pixel(x, y, dsc.shadow_color, final_opa);
        }
    }
}

fn shadow_coverage(x: i32, y: i32, rect: &Area, radius: i32, width: i32) -> Opa {
    let cx = x.clamp(rect.x1, rect.x2);
    let cy = y.clamp(rect.y1, rect.y2);

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
