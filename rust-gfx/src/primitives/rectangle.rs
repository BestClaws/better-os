/// Rectangle drawing matching LVGL's lv_draw_rect functionality
/// Supports: solid fills, gradients, borders, shadows, outlines, rounded corners
use crate::Rasterizer;
use crate::color::Rgba8888;
use crate::types::*;
use crate::primitives::{gradient::*, mask::RadiusMask};
use crate::math::{aa_coverage_sq, dist_sq, isqrt};

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
        draw_bg(rast, dsc, area);
    }

    // Draw border (if any)
    if dsc.border_opa > 0 && dsc.border_width > 0 {
        draw_border(rast, dsc, area);
    }
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

            // LVGL writes ALL pixels in the rect, even if mask==0
            // This preserves color info in transparent pixels (non-premultiplied alpha)

            // Get color (possibly from gradient)
            let (color, grad_opa) = if has_grad {
                let rel_x = x - area.x1;
                let rel_y = y - area.y1;
                gradient_get_color(&dsc.bg_grad, rel_x, rel_y, width, height, width / 2, height / 2)
            } else {
                (dsc.bg_color, OPA_COVER)
            };

            // Combine opacities: dsc.bg_opa * grad_opa * mask_val
            let opa = ((dsc.bg_opa as u32 * grad_opa as u32 * mask_val as u32) / (255 * 255)) as Opa;

            // Write pixel even if opa==0 (LVGL behavior for non-premultiplied alpha)
            rast.blend_pixel(x, y, color, opa);
        }
    }
}

/// Draw rectangle border
fn draw_border<R: Rasterizer>(rast: &mut R, dsc: &RectDsc, area: &Area) {
    let width = area.width();
    let height = area.height();
    
    if width <= 0 || height <= 0 {
        return;
    }

    let short_side = width.min(height);
    let mut radius = dsc.radius.min(short_side / 2);
    
    if dsc.radius == RADIUS_CIRCLE {
        radius = short_side / 2;
    }

    let bw = dsc.border_width;
    let sides = dsc.border_side;

    // Draw each side
    for y in area.y1..=area.y2 {
        for x in area.x1..=area.x2 {
            let rel_x = x - area.x1;
            let rel_y = y - area.y1;
            let rel_x_end = area.x2 - x;
            let rel_y_end = area.y2 - y;

            let mut is_border = false;

            // Check if this pixel is on a border edge
            if sides.has_top() && rel_y < bw {
                is_border = true;
            }
            if sides.has_bottom() && rel_y_end < bw {
                is_border = true;
            }
            if sides.has_left() && rel_x < bw {
                is_border = true;
            }
            if sides.has_right() && rel_x_end < bw {
                is_border = true;
            }

            if !is_border {
                continue;
            }

            // Apply radius masking if needed
            let mask_val = if radius > 0 {
                // For borders, we need to check if we're in the border ring
                let outer_mask = RadiusMask::new(*area, radius, false);
                let inner_area = Area::new(
                    area.x1 + bw,
                    area.y1 + bw,
                    area.x2 - bw,
                    area.y2 - bw,
                );
                let inner_radius = (radius - bw).max(0);
                let inner_mask = RadiusMask::new(inner_area, inner_radius, true);

                let outer_val = outer_mask.get_mask_value(x, y);
                let inner_val = inner_mask.get_mask_value(x, y);

                // Border is where outer is visible but inner is not
                ((outer_val as u32 * inner_val as u32) / 255) as Opa
            } else {
                OPA_COVER
            };

            // Draw even if mask_val == 0 to preserve color info (non-premultiplied alpha)
            rast.blend_pixel(x, y, dsc.border_color, ((dsc.border_opa as u32 * mask_val as u32) / 255) as Opa);
        }
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
            let shadow_opa = calculate_shadow_opa(
                x, y, area, shadow_radius, dsc.shadow_width
            );

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
            let in_outer = is_point_in_rounded_rect(x, y, &outline_area, outline_radius + dsc.outline_width);
            let in_inner = is_point_in_rounded_rect(x, y, &inner_area, outline_radius);

            // Draw with appropriate opacity (0 if outside outline ring)
            let opa = if in_outer && !in_inner { dsc.outline_opa } else { 0 };
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
