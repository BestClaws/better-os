// file: src/shapes/rounded_rect.rs

use crate::color::Rgba8888;
use crate::{
    aa_coverage, linear_gradient_h_rgba, linear_gradient_v_rgba, radial_gradient_rgba_sq, Fill,
    Rasterizer,
};

pub struct RoundedRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius_tl: i32,
    radius_tr: i32,
    radius_bl: i32,
    radius_br: i32,
    stroke_width: i32,
    stroke_color: Rgba8888,
    stroke_alpha: u8,
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

impl super::Shape for RoundedRect {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        // Refactored: compute coverage and colors, then delegate blending to rasterizer
        let x1 = self.x;
        let y1 = self.y;
        let x2 = self.x + self.width - 1;
        let y2 = self.y + self.height - 1;

        let outer = 1;

        let min_x = (x1 - outer).max(0);
        let max_x = (x2 + outer).min(rasterizer.width() as i32 - 1);
        let min_y = (y1 - outer).max(0);
        let max_y = (y2 + outer).min(rasterizer.height() as i32 - 1);

        let inner_x1 = x1 + self.stroke_width;
        let inner_x2 = x2 - self.stroke_width;
        let inner_y1 = y1 + self.stroke_width;
        let inner_y2 = y2 - self.stroke_width;

        let inner_radius_tl = (self.radius_tl - self.stroke_width).max(0);
        let inner_radius_tr = (self.radius_tr - self.stroke_width).max(0);
        let inner_radius_bl = (self.radius_bl - self.stroke_width).max(0);
        let inner_radius_br = (self.radius_br - self.stroke_width).max(0);

        // No RGB565 conversions in primitives; rasterizer handles native formats.

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                // Stroke
                if self.stroke_width > 0 {
                    let mut stroke_opa = 0u8;
                    let mut in_corner = false;
                    let mut dist2: i32 = 0;
                    let mut outer_r: i32 = 0;
                    let mut inner_r: i32 = 0;

                    if px <= x1 + self.radius_tl && py <= y1 + self.radius_tl {
                        let cx = x1 + self.radius_tl;
                        let cy = y1 + self.radius_tl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        outer_r = self.radius_tl;
                        inner_r = inner_radius_tl;
                        in_corner = true;
                    } else if px >= x2 - self.radius_tr && py <= y1 + self.radius_tr {
                        let cx = x2 - self.radius_tr;
                        let cy = y1 + self.radius_tr;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        outer_r = self.radius_tr;
                        inner_r = inner_radius_tr;
                        in_corner = true;
                    } else if px <= x1 + self.radius_bl && py >= y2 - self.radius_bl {
                        let cx = x1 + self.radius_bl;
                        let cy = y2 - self.radius_bl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        outer_r = self.radius_bl;
                        inner_r = inner_radius_bl;
                        in_corner = true;
                    } else if px >= x2 - self.radius_br && py >= y2 - self.radius_br {
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
                        if dist2 >= inner_r * inner_r && dist2 <= (outer_r + 1) * (outer_r + 1) {
                            stroke_opa = aa_coverage(dist2, outer_r);
                        }
                    } else {
                        if (py >= y1
                            && py < y1 + self.stroke_width
                            && px >= x1 + self.radius_tl
                            && px <= x2 - self.radius_tr)
                            || (py > inner_y2
                                && py <= y2
                                && px >= x1 + self.radius_bl
                                && px <= x2 - self.radius_br)
                            || (px >= x1
                                && px < x1 + self.stroke_width
                                && py >= y1 + self.radius_tl
                                && py <= y2 - self.radius_bl)
                            || (px > inner_x2
                                && px <= x2
                                && py >= y1 + self.radius_tr
                                && py <= y2 - self.radius_br)
                        {
                            stroke_opa = 255;
                        }
                    }

                    if stroke_opa > 0 {
                        let su = self.stroke_color.to_u32();
                        let sr = ((su >> 24) & 0xFF) as u8;
                        let sg = ((su >> 16) & 0xFF) as u8;
                        let sb = ((su >> 8) & 0xFF) as u8;
                        let sa = (su & 0xFF) as u8;
                        let sa_eff = ((sa as u32 * self.stroke_alpha as u32) / 255) as u8;
                        let stroke_rgba = Rgba8888::rgba(sr, sg, sb, sa_eff);
                        rasterizer.blend_pixel(px, py, stroke_rgba, stroke_opa);
                    }
                }

                // Fill
                if let Some(fill) = self.fill {
                    let mut fill_opa = 0u8;
                    let mut in_corner = false;
                    let mut dist2: i32 = 0;
                    let mut r: i32 = 0;

                    if px <= x1 + self.radius_tl && py <= y1 + self.radius_tl {
                        let cx = x1 + self.radius_tl;
                        let cy = y1 + self.radius_tl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        r = inner_radius_tl;
                        in_corner = true;
                    } else if px >= x2 - self.radius_tr && py <= y1 + self.radius_tr {
                        let cx = x2 - self.radius_tr;
                        let cy = y1 + self.radius_tr;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        r = inner_radius_tr;
                        in_corner = true;
                    } else if px <= x1 + self.radius_bl && py >= y2 - self.radius_bl {
                        let cx = x1 + self.radius_bl;
                        let cy = y2 - self.radius_bl;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
                        r = inner_radius_bl;
                        in_corner = true;
                    } else if px >= x2 - self.radius_br && py >= y2 - self.radius_br {
                        let cx = x2 - self.radius_br;
                        let cy = y2 - self.radius_br;
                        let dx = px - cx;
                        let dy = py - cy;
                        dist2 = dx * dx + dy * dy;
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
                        let color = match fill {
                            Fill::Solid(c) => c,
                            Fill::RadialGradient { inner, outer } => {
                                let cx = self.x + self.width / 2;
                                let cy = self.y + self.height / 2;
                                let dx = px - cx;
                                let dy = py - cy;
                                let dist2 = dx * dx + dy * dy;
                                let r2 = (self.width / 2).pow(2) + (self.height / 2).pow(2);
                                radial_gradient_rgba_sq(inner, outer, dist2, r2)
                            }
                            Fill::LinearGradientH { start, end } => linear_gradient_h_rgba(
                                start,
                                end,
                                px,
                                self.x + self.width / 2,
                                self.width / 2,
                            ),
                            Fill::LinearGradientV { start, end } => linear_gradient_v_rgba(
                                start,
                                end,
                                py,
                                self.y + self.height / 2,
                                self.height / 2,
                            ),
                        };
                        rasterizer.blend_pixel(px, py, color, fill_opa);
                    }
                }
            }
        }

        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}
