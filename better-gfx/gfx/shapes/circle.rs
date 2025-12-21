// file: src/shapes/circle.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{aa_coverage, angle_in_range, blend_rgb565, linear_gradient_h, linear_gradient_v, radial_gradient_sq, rgba8888_to_rgb565_and_alpha, Fill, Rasterizer};

pub struct Circle {
    cx: i32,
    cy: i32,
    radius: i32,
    stroke_width: i32,
    stroke_color: u16,
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
            stroke_color: 0,
            stroke_alpha: 255,
            fill: None,
        }
    }

    pub fn stroke(mut self, width: i32, color: Rgba8888) -> Self {
        let (rgb565, alpha) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        self.stroke_width = width;
        self.stroke_color = rgb565;
        self.stroke_alpha = alpha;
        self
    }

    pub fn stroke_alpha(mut self, alpha: u8) -> Self {
        self.stroke_alpha = alpha;
        self
    }

    pub fn fill_solid(mut self, color: Rgba8888) -> Self {
        let (rgb565, alpha) = rgba8888_to_rgb565_and_alpha(color.to_u32());
        self.fill = Some(Fill::Solid(rgb565, alpha));
        self
    }

    pub fn fill_radial(mut self, inner: Rgba8888, outer: Rgba8888) -> Self {
        let (inner_color, inner_alpha) = rgba8888_to_rgb565_and_alpha(inner.to_u32());
        let (outer_color, outer_alpha) = rgba8888_to_rgb565_and_alpha(outer.to_u32());
        self.fill = Some(Fill::RadialGradient {
            inner_color,
            inner_alpha,
            outer_color,
            outer_alpha,
        });
        self
    }

    pub fn fill_linear_h(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let (start_color, start_alpha) = rgba8888_to_rgb565_and_alpha(start.to_u32());
        let (end_color, end_alpha) = rgba8888_to_rgb565_and_alpha(end.to_u32());
        self.fill = Some(Fill::LinearGradientH {
            start_color,
            start_alpha,
            end_color,
            end_alpha,
        });
        self
    }

    pub fn fill_linear_v(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let (start_color, start_alpha) = rgba8888_to_rgb565_and_alpha(start.to_u32());
        let (end_color, end_alpha) = rgba8888_to_rgb565_and_alpha(end.to_u32());
        self.fill = Some(Fill::LinearGradientV {
            start_color,
            start_alpha,
            end_color,
            end_alpha,
        });
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
    stroke_color: u16,
    stroke_alpha: u8,
    fill: Option<Fill>,
    arc: Option<(i32, i32)>,
) {
    // (exact original implementation from your provided circle.rs - unchanged)
    // ... [paste the full original draw_circle_or_arc body here]
    let width = rasterizer.width() as usize;
    let r_fill = radius;
    let r_outer = radius + stroke_width;

    let r_fill2 = r_fill * r_fill;
    let r_outer2_aa = (r_outer + 1) * (r_outer + 1);
    let r_fill2_aa = (r_fill + 1) * (r_fill + 1);

    let min_x = (cx - r_outer - 1).max(0);
    let max_x = (cx + r_outer + 1).min(rasterizer.width() as i32 - 1);
    let min_y = (cy - r_outer - 1).max(0);
    let max_y = (cy + r_outer + 1).min(rasterizer.height() as i32 - 1);

    let mut buf = rasterizer.buffer_mut();

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

                let effective_opa = ((opa as u32 * stroke_alpha as u32) / 255) as u8;
                if effective_opa == 0 {
                    continue;
                }


                let idx = (y as usize * width + x as usize) * 2;
                let bg = ((buf[idx] as u16) << 8) | buf[idx + 1] as u16;

                let out = blend_rgb565(bg, stroke_color, effective_opa);
                buf[idx] = (out >> 8) as u8;
                buf[idx + 1] = out as u8;
            }
        }
    }

    if let Some(fill) = fill {
        for y in min_y..=max_y {
            let dy = y - cy;
            let dy2 = dy * dy;

            for x in min_x..=max_x {
                let dx = x - cx;
                let dist2 = dx * dx + dy2;

                if dist2 > r_fill2_aa {
                    continue;
                }

                if let Some((s, e)) = arc {
                    if !angle_in_range(dx, dy, s, e) {
                        continue;
                    }
                }

                let opa = aa_coverage(dist2, r_fill);
                if opa == 0 {
                    continue;
                }

                let (color, px_alpha) = match fill {
                    Fill::Solid(c, a) => (c, a),
                    Fill::RadialGradient {
                        inner_color,
                        outer_color,
                        inner_alpha,
                        outer_alpha,
                    } => radial_gradient_sq(inner_color, outer_color, inner_alpha, outer_alpha, dist2, r_fill2),
                    Fill::LinearGradientH {
                        start_color,
                        end_color,
                        start_alpha,
                        end_alpha,
                    } => linear_gradient_h(start_color, end_color, start_alpha, end_alpha, x, cx, r_fill),
                    Fill::LinearGradientV {
                        start_color,
                        end_color,
                        start_alpha,
                        end_alpha,
                    } => linear_gradient_v(start_color, end_color, start_alpha, end_alpha, y, cy, r_fill),
                };

                let effective_opa = ((opa as u32 * px_alpha as u32) / 255) as u8;
                if effective_opa == 0 {
                    continue;
                }

                let idx = (y as usize * width + x as usize) * 2;
                let bg = ((buf[idx] as u16) << 8) | buf[idx + 1] as u16;

                let out = blend_rgb565(bg, color, effective_opa);
                buf[idx] = (out >> 8) as u8;
                buf[idx + 1] = out as u8;
            }
        }
    }

    rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
}