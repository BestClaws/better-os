use crate::types::{Area, BorderSide};
use crate::Rasterizer;

use super::border::draw_stroke;
use super::descriptor::RectDsc;

/// Compute the bounds covered by the outline effect.
pub fn outline_bounds(area: &Area, dsc: &RectDsc) -> Area {
    let ext = dsc.outline_pad + dsc.outline_width;
    Area::new(area.x1 - ext, area.y1 - ext, area.x2 + ext, area.y2 + ext)
}

/// Render the outline by reusing the border stroke machinery.
pub fn draw_outline<R: Rasterizer>(
    rast: &mut R,
    dsc: &RectDsc,
    area: &Area,
    clip: Option<Area>,
) {
    if dsc.outline_opa == 0 || dsc.outline_width <= 0 {
        return;
    }

    let ext = dsc.outline_pad + dsc.outline_width;
    let outer = Area::new(area.x1 - ext, area.y1 - ext, area.x2 + ext, area.y2 + ext);
    let raw_radius = if dsc.radius == crate::types::RADIUS_CIRCLE {
        crate::types::RADIUS_CIRCLE
    } else {
        dsc.radius + ext
    };

    draw_stroke(
        rast,
        &outer,
        raw_radius,
        dsc.outline_width,
        BorderSide::FULL,
        dsc.outline_color,
        dsc.outline_opa,
        clip.as_ref(),
    );
}
