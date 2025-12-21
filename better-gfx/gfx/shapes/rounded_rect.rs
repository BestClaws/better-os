// file: src/shapes/rounded_rect.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{aa_coverage, blend_rgb565, linear_gradient_h, linear_gradient_v, radial_gradient_sq, rgba8888_to_rgb565_and_alpha, Fill, Rasterizer};

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
    stroke_color: u16,
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

impl super::Shape for RoundedRect {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        // (exact original implementation from your provided rounded_rect.rs - unchanged except removed fill_opacity)
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

        let width = rasterizer.width() as usize;

        let mut buf = rasterizer.buffer_mut();

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let idx = (py as usize * width + px as usize) * 2;
                let bg = ((buf[idx] as u16) << 8) | buf[idx + 1] as u16;
                let mut color_out = bg;

                // Stroke
                let mut stroke_opa = 0u8;
                if self.stroke_width > 0 {
                    // (original stroke logic unchanged)
                    // ... [full original stroke corner and edge detection]
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
                        if (py >= y1 && py < y1 + self.stroke_width && px >= x1 + self.radius_tl && px <= x2 - self.radius_tr) ||
                            (py > inner_y2 && py <= y2 && px >= x1 + self.radius_bl && px <= x2 - self.radius_br) ||
                            (px >= x1 && px < x1 + self.stroke_width && py >= y1 + self.radius_tl && py <= y2 - self.radius_bl) ||
                            (px > inner_x2 && px <= x2 && py >= y1 + self.radius_tr && py <= y2 - self.radius_br) {
                            stroke_opa = 255;
                        }
                    }

                    if stroke_opa > 0 {
                        let effective_opa = ((stroke_opa as u32 * self.stroke_alpha as u32) / 255) as u8;
                        if effective_opa > 0 {
                            color_out = blend_rgb565(color_out, self.stroke_color, effective_opa);
                        }
                    }
                }

                // Fill
                if let Some(fill) = self.fill {
                    let mut fill_opa = 0u8;
                    // (original fill logic unchanged)
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
                        let (color, px_alpha) = match fill {
                            Fill::Solid(c, a) => (c, a),
                            Fill::RadialGradient {
                                inner_color,
                                outer_color,
                                inner_alpha,
                                outer_alpha,
                            } => {
                                let cx = self.x + self.width / 2;
                                let cy = self.y + self.height / 2;
                                let dx = px - cx;
                                let dy = py - cy;
                                let dist2 = dx * dx + dy * dy;
                                let r2 = (self.width / 2).pow(2) + (self.height / 2).pow(2);
                                radial_gradient_sq(inner_color, outer_color, inner_alpha, outer_alpha, dist2, r2)
                            }
                            Fill::LinearGradientH {
                                start_color,
                                end_color,
                                start_alpha,
                                end_alpha,
                            } => linear_gradient_h(start_color, end_color, start_alpha, end_alpha, px, self.x + self.width / 2, self.width / 2),
                            Fill::LinearGradientV {
                                start_color,
                                end_color,
                                start_alpha,
                                end_alpha,
                            } => linear_gradient_v(start_color, end_color, start_alpha, end_alpha, py, self.y + self.height / 2, self.height / 2),
                        };
                        let effective_opa = ((fill_opa as u32 * px_alpha as u32) / 255) as u8;
                        if effective_opa > 0 {
                            color_out = blend_rgb565(color_out, color, effective_opa);
                        }
                    }
                }

                buf[idx] = (color_out >> 8) as u8;
                buf[idx + 1] = color_out as u8;
            }
        }

        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}