use crate::color::Rgba8888;
use crate::types::{Area, OPA_50, OPA_COVER};
/// Blur effect (simplified stub for now)
use crate::Rasterizer;

/// Blur descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct BlurDsc {
    pub blur_radius: i32,
    pub corner_radius: i32,
}

impl BlurDsc {
    pub fn new(blur_radius: i32) -> Self {
        Self {
            blur_radius,
            corner_radius: 0,
        }
    }
}

/// Apply blur effect (stub - draws diagnostic overlay for now)
pub fn draw_blur<R: Rasterizer>(rast: &mut R, dsc: &BlurDsc, area: &Area) {
    // Placeholder: draw cross-hatch overlay to indicate missing blur
    // Uses blur_radius to vary spacing, corner_radius currently ignored
    let spacing = (dsc.blur_radius.max(1) as usize).min(12);
    let overlay = Rgba8888::rgba(255, 0, 255, 45);
    let highlight = Rgba8888::rgba(255, 255, 255, 40);

    for y in area.y1..=area.y2 {
        for x in area.x1..=area.x2 {
            let pattern = ((x + y) as usize / spacing) % 2 == 0;
            let color = if pattern { overlay } else { highlight };
            rast.blend_pixel(x, y, color, if pattern { OPA_50 } else { OPA_COVER / 6 });
        }
    }

    // Draw subtle outline to match LVGL style cues
    let border = Rgba8888::rgba(200, 0, 200, 120);
    for x in area.x1..=area.x2 {
        rast.blend_pixel(x, area.y1, border, OPA_50);
        rast.blend_pixel(x, area.y2, border, OPA_50);
    }
    for y in area.y1..=area.y2 {
        rast.blend_pixel(area.x1, y, border, OPA_50);
        rast.blend_pixel(area.x2, y, border, OPA_50);
    }

    rast.mark_dirty(area.x1, area.y1, area.x2 + 1, area.y2 + 1);
}
