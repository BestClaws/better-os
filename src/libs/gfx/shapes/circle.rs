// file: src/shapes/circle.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{
    aa_coverage, angle_in_range, linear_gradient_h_rgba, linear_gradient_v_rgba,
    radial_gradient_rgba_sq, Fill, Rasterizer,
};

pub struct Circle {
    cx: i32,
    cy: i32,
    radius: i32,
    stroke_width: i32,
    stroke_color: Rgba8888,
    stroke_alpha: u8,
    fill: Option<Fill>,
}

impl Circle {
    pub fn new(cx: i32, cy: i32, radius: i32) -> Self {
        Self {
            cx,
            cy,
            radius,
            stroke_width: 0,
            stroke_color: Rgba8888::rgba(0, 0, 0, 255),
            stroke_alpha: 255,
            fill: None,
        }
    }

    pub fn stroke(mut self, width: i32, color: Rgba8888) -> Self {
        self.stroke_width = width;
        self.stroke_color = color;
        self
    }

    pub fn stroke_alpha(mut self, alpha: u8) -> Self {
        self.stroke_alpha = alpha;
        self
    }

    pub fn fill_solid(mut self, color: Rgba8888) -> Self {
        self.fill = Some(Fill::Solid(color));
        self
    }

    pub fn fill_radial(mut self, inner: Rgba8888, outer: Rgba8888) -> Self {
        self.fill = Some(Fill::RadialGradient { inner, outer });
        self
    }

    pub fn fill_linear_h(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        self.fill = Some(Fill::LinearGradientH { start, end });
        self
    }

    pub fn fill_linear_v(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        self.fill = Some(Fill::LinearGradientV { start, end });
        self
    }
}

impl super::Shape for Circle {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        crate::libs::gfx::shapes::circle::draw_circle_or_arc(
            rasterizer,
            self.cx,
            self.cy,
            self.radius,
            self.stroke_width,
            self.stroke_color,
            self.stroke_alpha,
            self.fill,
            None,
        );
    }
}

// draw_circle_or_arc remains unchanged (moved to its own file or kept here - original code preserved)
pub(crate) fn draw_circle_or_arc<R: Rasterizer>(
    rasterizer: &mut R,
    cx: i32,
    cy: i32,
    radius: i32,
    stroke_width: i32,
    stroke_color: Rgba8888,
    stroke_alpha: u8,
    fill: Option<Fill>,
    arc: Option<(i32, i32)>,
) {
    // Refactored to use rasterizer blend APIs
    let r_fill = radius;
    let r_outer = radius + stroke_width;

    let r_fill2 = r_fill * r_fill;
    let r_outer2_aa = (r_outer + 1) * (r_outer + 1);
    let r_fill2_aa = (r_fill + 1) * (r_fill + 1);

    let min_x = (cx - r_outer - 1).max(0);
    let max_x = (cx + r_outer + 1).min(rasterizer.width() as i32 - 1);
    let min_y = (cy - r_outer - 1).max(0);
    let max_y = (cy + r_outer + 1).min(rasterizer.height() as i32 - 1);

    // No RGB565 conversions in primitives: rasterizer handles native formats.

    if stroke_width > 0 {
        for y in min_y..=max_y {
            let dy = y - cy;
            let dy2 = dy * dy;

            for x in min_x..=max_x {
                let dx = x - cx;
                let dist2 = dx * dx + dy2;

                if dist2 < r_fill2 || dist2 > r_outer2_aa {
                    continue;
                }

                if let Some((s, e)) = arc {
                    if !angle_in_range(dx, dy, s, e) {
                        continue;
                    }
                }

                let opa = aa_coverage(dist2, r_outer);
                if opa == 0 {
                    continue;
                }

                // Adjust stroke alpha by stroke_alpha multiplier
                let su = stroke_color.to_u32();
                let sr = ((su >> 24) & 0xFF) as u8;
                let sg = ((su >> 16) & 0xFF) as u8;
                let sb = ((su >> 8) & 0xFF) as u8;
                let sa = (su & 0xFF) as u8;
                let sa_eff = ((sa as u32 * stroke_alpha as u32) / 255) as u8;
                let stroke_rgba = Rgba8888::rgba(sr, sg, sb, sa_eff);
                rasterizer.blend_pixel(x, y, stroke_rgba, opa);
            }
        }
    }

    if let Some(fill) = fill {
        for y in min_y..=max_y {
            let dy = y - cy;
            let dy2 = dy * dy;
            // Use a horizontal span with a closure to compute color + coverage per pixel
            let len = (max_x - min_x + 1).max(0);
            rasterizer.blend_hspan_with(min_x, y, len, |i| {
                let x = min_x + i as i32;
                let dx = x - cx;
                let dist2 = dx * dx + dy2;
                if dist2 > r_fill2_aa {
                    return (Rgba8888::rgba(0, 0, 0, 0), 0);
                }
                if let Some((s, e)) = arc {
                    if !angle_in_range(dx, dy, s, e) {
                        return (Rgba8888::rgba(0, 0, 0, 0), 0);
                    }
                }
                let opa = aa_coverage(dist2, r_fill);
                if opa == 0 {
                    return (Rgba8888::rgba(0, 0, 0, 0), 0);
                }
                let color = match fill {
                    Fill::Solid(c) => c,
                    Fill::RadialGradient { inner, outer } => {
                        radial_gradient_rgba_sq(inner, outer, dist2, r_fill2)
                    }
                    Fill::LinearGradientH { start, end } => {
                        linear_gradient_h_rgba(start, end, x, cx, r_fill)
                    }
                    Fill::LinearGradientV { start, end } => {
                        linear_gradient_v_rgba(start, end, y, cy, r_fill)
                    }
                };
                (color, opa)
            });
        }
    }

    rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
}
