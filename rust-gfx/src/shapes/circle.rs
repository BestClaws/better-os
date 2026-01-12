// file: src/shapes/circle.rs

use crate::color::Rgba8888;
use crate::{aa_coverage, angle_in_range, Fill, FillContext, Rasterizer, StrokeStyle};

pub struct Circle {
    cx: i32,
    cy: i32,
    radius: i32,
    stroke: StrokeStyle,
    fill: Option<Fill>,
}

impl Circle {
    pub fn new(cx: i32, cy: i32, radius: i32) -> Self {
        Self {
            cx,
            cy,
            radius,
            stroke: StrokeStyle::disabled(),
            fill: None,
        }
    }

    pub fn stroke(mut self, width: i32, color: Rgba8888) -> Self {
        self.stroke.set(width, color);
        self
    }

    pub fn stroke_alpha(mut self, alpha: u8) -> Self {
        self.stroke.set_alpha(alpha);
        self
    }

    pub fn fill_solid(mut self, color: Rgba8888) -> Self {
        self.fill = Some(Fill::solid(color));
        self
    }

    pub fn fill_radial(mut self, inner: Rgba8888, outer: Rgba8888) -> Self {
        let radius = self.radius.max(0) as i64;
        let radius_sq = (radius * radius).max(1);
        self.fill = Some(Fill::radial(self.cx, self.cy, radius_sq, inner, outer));
        self
    }

    pub fn fill_linear_h(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let origin = self.cx - self.radius;
        let length = (self.radius.max(0)).saturating_mul(2).max(1);
        self.fill = Some(Fill::linear_horizontal(start, end, origin, length));
        self
    }

    pub fn fill_linear_v(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let origin = self.cy - self.radius;
        let length = (self.radius.max(0)).saturating_mul(2).max(1);
        self.fill = Some(Fill::linear_vertical(start, end, origin, length));
        self
    }
}

impl super::Shape for Circle {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        draw_circle_or_arc(
            rasterizer,
            self.cx,
            self.cy,
            self.radius,
            self.stroke,
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
    stroke: StrokeStyle,
    fill: Option<Fill>,
    arc: Option<(i32, i32)>,
) {
    let stroke_color = stroke.effective_color();
    let stroke_width = if stroke_color.is_some() {
        stroke.width()
    } else {
        0
    };

    let r_fill = radius.max(0);
    let r_outer = r_fill + stroke_width.max(0);

    let r_fill2 = r_fill * r_fill;
    let r_outer2_aa = (r_outer + 1) * (r_outer + 1);
    let r_fill2_aa = (r_fill + 1) * (r_fill + 1);

    let width = rasterizer.width() as i32;
    let height = rasterizer.height() as i32;

    let min_x = (cx - r_outer - 1).max(0);
    let max_x = (cx + r_outer + 1).min(width - 1);
    let min_y = (cy - r_outer - 1).max(0);
    let max_y = (cy + r_outer + 1).min(height - 1);

    if let Some(stroke_rgba) = stroke_color {
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

                    if let Some((start_deg, end_deg)) = arc {
                        if !angle_in_range(dx, dy, start_deg, end_deg) {
                            continue;
                        }
                    }

                    let opa = aa_coverage(dist2, r_outer);
                    if opa == 0 {
                        continue;
                    }

                    rasterizer.blend_pixel(x, y, stroke_rgba, opa);
                }
            }
        }
    }

    if let Some(fill_style) = fill {
        for y in min_y..=max_y {
            let dy = y - cy;
            let dy2 = dy * dy;
            let span = (max_x - min_x + 1).max(0);
            rasterizer.blend_hspan_with(min_x, y, span, |i| {
                let x = min_x + i as i32;
                let dx = x - cx;
                let dist2 = dx * dx + dy2;
                if dist2 > r_fill2_aa {
                    return (Rgba8888::TRANSPARENT, 0);
                }
                if let Some((start_deg, end_deg)) = arc {
                    if !angle_in_range(dx, dy, start_deg, end_deg) {
                        return (Rgba8888::TRANSPARENT, 0);
                    }
                }
                let opa = aa_coverage(dist2, r_fill);
                if opa == 0 {
                    return (Rgba8888::TRANSPARENT, 0);
                }
                let ctx = FillContext::with_distance(x, y, dist2);
                (fill_style.shade(ctx), opa)
            });
        }
    }

    if stroke_color.is_some() || fill.is_some() {
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}
