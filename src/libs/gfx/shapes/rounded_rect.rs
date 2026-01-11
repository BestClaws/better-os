// file: src/shapes/rounded_rect.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{aa_coverage, Fill, FillContext, Rasterizer, StrokeStyle};

pub struct RoundedRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius_tl: i32,
    radius_tr: i32,
    radius_bl: i32,
    radius_br: i32,
    stroke: StrokeStyle,
    fill: Option<Fill>,
}

impl RoundedRect {
    pub fn new(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        radius_tl: i32,
        radius_tr: i32,
        radius_bl: i32,
        radius_br: i32,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            radius_tl,
            radius_tr,
            radius_bl,
            radius_br,
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
        let cx = self.x + self.width / 2;
        let cy = self.y + self.height / 2;
        let rx = (self.width / 2).max(1);
        let ry = (self.height / 2).max(1);
        let radius_sq = (rx as i64 * rx as i64 + ry as i64 * ry as i64).max(1);
        self.fill = Some(Fill::radial(cx, cy, radius_sq, inner, outer));
        self
    }

    pub fn fill_linear_h(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let length = self.width.max(1);
        self.fill = Some(Fill::linear_horizontal(start, end, self.x, length));
        self
    }

    pub fn fill_linear_v(mut self, start: Rgba8888, end: Rgba8888) -> Self {
        let length = self.height.max(1);
        self.fill = Some(Fill::linear_vertical(start, end, self.y, length));
        self
    }
}

impl super::Shape for RoundedRect {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        // Refactored: compute coverage and colors, then delegate blending to rasterizer
        let x1 = self.x;
        let y1 = self.y;
        let x2 = self.x + self.width - 1;
        let y2 = self.y + self.height - 1;

        let stroke_color = self.stroke.effective_color();
        let stroke_width = if stroke_color.is_some() {
            self.stroke.width()
        } else {
            0
        };

        let outer = 1;

        let min_x = (x1 - outer).max(0);
        let max_x = (x2 + outer).min(rasterizer.width() as i32 - 1);
        let min_y = (y1 - outer).max(0);
        let max_y = (y2 + outer).min(rasterizer.height() as i32 - 1);

        let inner_x1 = x1 + stroke_width;
        let inner_x2 = x2 - stroke_width;
        let inner_y1 = y1 + stroke_width;
        let inner_y2 = y2 - stroke_width;

        let inner_radius_tl = (self.radius_tl - stroke_width).max(0);
        let inner_radius_tr = (self.radius_tr - stroke_width).max(0);
        let inner_radius_bl = (self.radius_bl - stroke_width).max(0);
        let inner_radius_br = (self.radius_br - stroke_width).max(0);

        // No RGB565 conversions in primitives; rasterizer handles native formats.

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                // Stroke
                if stroke_width > 0 {
                    if let Some(stroke_rgba) = stroke_color {
                        let mut stroke_opa = 0u8;
                        let mut in_corner = false;
                        let mut dist2: i32 = 0;
                        let mut outer_r: i32 = 0;
                        let mut inner_r: i32 = 0;

                        if self.radius_tl > 0
                            && px <= x1 + self.radius_tl
                            && py <= y1 + self.radius_tl
                        {
                            let cx = x1 + self.radius_tl;
                            let cy = y1 + self.radius_tl;
                            let dx = px - cx;
                            let dy = py - cy;
                            dist2 = dx * dx + dy * dy;
                            outer_r = self.radius_tl;
                            inner_r = inner_radius_tl;
                            in_corner = true;
                        } else if self.radius_tr > 0
                            && px >= x2 - self.radius_tr
                            && py <= y1 + self.radius_tr
                        {
                            let cx = x2 - self.radius_tr;
                            let cy = y1 + self.radius_tr;
                            let dx = px - cx;
                            let dy = py - cy;
                            dist2 = dx * dx + dy * dy;
                            outer_r = self.radius_tr;
                            inner_r = inner_radius_tr;
                            in_corner = true;
                        } else if self.radius_bl > 0
                            && px <= x1 + self.radius_bl
                            && py >= y2 - self.radius_bl
                        {
                            let cx = x1 + self.radius_bl;
                            let cy = y2 - self.radius_bl;
                            let dx = px - cx;
                            let dy = py - cy;
                            dist2 = dx * dx + dy * dy;
                            outer_r = self.radius_bl;
                            inner_r = inner_radius_bl;
                            in_corner = true;
                        } else if self.radius_br > 0
                            && px >= x2 - self.radius_br
                            && py >= y2 - self.radius_br
                        {
                            let cx = x2 - self.radius_br;
                            let cy = y2 - self.radius_br;
                            let dx = px - cx;
                            let dy = py - cy;
                            dist2 = dx * dx + dy * dy;
                            outer_r = self.radius_br;
                            inner_r = inner_radius_br;
                            in_corner = true;
                        }

                        if in_corner {
                            let cov_out = aa_coverage(dist2, outer_r);
                            let cov_in = aa_coverage(dist2, inner_r);
                            stroke_opa = cov_out.saturating_sub(cov_in);
                        } else if (py >= y1
                            && py < y1 + stroke_width
                            && px >= x1 + self.radius_tl
                            && px <= x2 - self.radius_tr)
                            || (py > inner_y2
                                && py <= y2
                                && px >= x1 + self.radius_bl
                                && px <= x2 - self.radius_br)
                            || (px >= x1
                                && px < x1 + stroke_width
                                && py >= y1 + self.radius_tl
                                && py <= y2 - self.radius_bl)
                            || (px > inner_x2
                                && px <= x2
                                && py >= y1 + self.radius_tr
                                && py <= y2 - self.radius_br)
                        {
                            stroke_opa = 255;
                        }

                        if stroke_opa > 0 {
                            rasterizer.blend_pixel(px, py, stroke_rgba, stroke_opa);
                        }
                    }
                }

                // Fill
                if let Some(fill_style) = self.fill {
                    let mut fill_opa = 0u8;
                    let mut in_corner = false;
                    let mut dist2: i32 = 0;
                    let mut r: i32 = 0;
                    let mut dist_hint: Option<i32> = None;

                    if self.radius_tl > 0 && px <= x1 + self.radius_tl && py <= y1 + self.radius_tl
                    {
                        let cx = x1 + self.radius_tl;
                        let cy = y1 + self.radius_tl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        dist_hint = Some(dist2);
                        r = inner_radius_tl;
                        in_corner = true;
                    } else if self.radius_tr > 0
                        && px >= x2 - self.radius_tr
                        && py <= y1 + self.radius_tr
                    {
                        let cx = x2 - self.radius_tr;
                        let cy = y1 + self.radius_tr;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        dist_hint = Some(dist2);
                        r = inner_radius_tr;
                        in_corner = true;
                    } else if self.radius_bl > 0
                        && px <= x1 + self.radius_bl
                        && py >= y2 - self.radius_bl
                    {
                        let cx = x1 + self.radius_bl;
                        let cy = y2 - self.radius_bl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        dist_hint = Some(dist2);
                        r = inner_radius_bl;
                        in_corner = true;
                    } else if self.radius_br > 0
                        && px >= x2 - self.radius_br
                        && py >= y2 - self.radius_br
                    {
                        let cx = x2 - self.radius_br;
                        let cy = y2 - self.radius_br;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        dist_hint = Some(dist2);
                        r = inner_radius_br;
                        in_corner = true;
                    }

                    if in_corner {
                        if dist2 <= (r + 1) * (r + 1) {
                            fill_opa = aa_coverage(dist2, r);
                        }
                    } else {
                        if px >= inner_x1 && px <= inner_x2 && py >= inner_y1 && py <= inner_y2 {
                            fill_opa = 255;
                        }
                    }

                    if fill_opa > 0 {
                        let ctx = if let Some(dist2) = dist_hint {
                            FillContext::with_distance(px, py, dist2)
                        } else {
                            FillContext::new(px, py)
                        };
                        let color = fill_style.shade(ctx);
                        rasterizer.blend_pixel(px, py, color, fill_opa);
                    }
                }
            }
        }

        if stroke_color.is_some() || self.fill.is_some() {
            rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
        }
    }
}
