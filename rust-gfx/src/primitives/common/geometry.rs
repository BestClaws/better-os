use crate::color::Rgba8888;
use crate::types::{Area, Opa, OPA_COVER, RADIUS_CIRCLE};
use crate::Rasterizer;

/// Intersect a draw area with the rasterizer bounds.
#[inline]
pub fn clip_to_raster<R: Rasterizer>(area: &Area, rast: &R) -> Option<Area> {
    if rast.width() == 0 || rast.height() == 0 {
        return None;
    }

    let bounds = Area::new(0, 0, rast.width() as i32 - 1, rast.height() as i32 - 1);
    area.intersect(&bounds)
}

/// Blend a solid rectangle while respecting the raster bounds.
pub fn fill_rect_with_clipping<R: Rasterizer>(
    rast: &mut R,
    rect: &Area,
    color: Rgba8888,
    opa: Opa,
) {
    if opa == 0 || rect.width() <= 0 || rect.height() <= 0 {
        return;
    }

    let Some(clamped) = clip_to_raster(rect, rast) else {
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
        for x in clamped.x1..=clamped.x2 {
            rast.blend_pixel(x, y, color, opa);
        }
    }
}

/// Combine the bounds of two areas.
#[inline]
pub fn merge_areas(a: Area, b: Area) -> Area {
    Area::new(
        a.x1.min(b.x1),
        a.y1.min(b.y1),
        a.x2.max(b.x2),
        a.y2.max(b.y2),
    )
}

/// Return a corner radius that fits inside the given area.
#[inline]
pub fn effective_radius(area: &Area, requested: i32) -> i32 {
    let short_side = area.width().min(area.height());
    if requested == RADIUS_CIRCLE {
        short_side / 2
    } else {
        requested.min(short_side / 2).max(0)
    }
}
