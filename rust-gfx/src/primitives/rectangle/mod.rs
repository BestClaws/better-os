use crate::types::Area;
use crate::Rasterizer;

pub use descriptor::RectDsc;

mod background;
mod border;
mod descriptor;
mod outline;
mod shadow;

use background::{background_fill_area, draw_background};
use border::draw_border;
use outline::{draw_outline, outline_bounds};
use shadow::{draw_shadow, shadow_bounds};
use crate::primitives::common::PrimitivePipeline;

/// Render a rectangle following LVGL's draw order: shadow → outline → fill → border.
pub fn draw_rect<R>(rast: &mut R, dsc: &RectDsc, coords: &Area)
where
    R: Rasterizer,
{
    if coords.width() <= 0 || coords.height() <= 0 {
        return;
    }

    let mut pipeline = PrimitivePipeline::new(rast);
    pipeline.include(coords);

    if dsc.shadow_opa > 0 && dsc.shadow_width > 0 {
        let shadow_area = shadow_bounds(coords, dsc);
        pipeline.include(&shadow_area);
        draw_shadow(pipeline.raster_mut(), dsc, coords);
    }

    if dsc.bg_opa > 0 {
        let bg_area = background_fill_area(coords, dsc);
        draw_background(pipeline.raster_mut(), dsc, &bg_area);
    }

    if dsc.border_opa > 0 && dsc.border_width > 0 && dsc.border_side != crate::types::BorderSide::NONE {
        draw_border(pipeline.raster_mut(), dsc, coords);
    }

    if dsc.outline_opa > 0 && dsc.outline_width > 0 {
        let outline_area = outline_bounds(coords, dsc);
        pipeline.include(&outline_area);
        draw_outline(pipeline.raster_mut(), dsc, coords);
    }

    pipeline.finish();
}
