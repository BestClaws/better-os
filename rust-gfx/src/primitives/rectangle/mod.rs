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
use outline::draw_outline;
use shadow::draw_shadow;

/// Render a rectangle following LVGL's draw order: shadow → outline → fill → border.
pub fn draw_rect<R>(rast: &mut R, dsc: &RectDsc, coords: &Area)
where
    R: Rasterizer,
{
    if coords.width() <= 0 || coords.height() <= 0 {
        return;
    }

    if dsc.shadow_opa > 0 && dsc.shadow_width > 0 {
        draw_shadow(rast, dsc, coords);
    }

    if dsc.bg_opa > 0 {
        let bg_area = background_fill_area(coords, dsc);
        draw_background(rast, dsc, &bg_area);
    }

    if dsc.border_opa > 0 && dsc.border_width > 0 && dsc.border_side != crate::types::BorderSide::NONE {
        draw_border(rast, dsc, coords);
    }

    if dsc.outline_opa > 0 && dsc.outline_width > 0 {
        draw_outline(rast, dsc, coords);
    }
}
