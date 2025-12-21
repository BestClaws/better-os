// file: src/shapes/line.rs

use alloc::vec;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{aa_coverage, Rasterizer};

pub struct Line {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    width: i32,
    color: Rgba8888,
    alpha: u8,
}

impl Line {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            width: 1,
            color: Rgba8888::rgba(0, 0, 0, 255),
            alpha: 255,
        }
    }

    pub fn stroke(mut self, width: i32, color: Rgba8888) -> Self {
        self.width = width;
        self.color = color;
        self
    }

    pub fn stroke_alpha(mut self, alpha: u8) -> Self {
        self.alpha = alpha;
        self
    }
}

impl super::Shape for Line {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        if self.width <= 0 {
            return;
        }

        let mut x1 = self.x1;
        let mut y1 = self.y1;
        let mut x2 = self.x2;
        let mut y2 = self.y2;

        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };

        let mut err = dx - dy;

        let width = rasterizer.width() as i32;
        let height = rasterizer.height() as i32;

        let half_width = self.width / 2;
        let outer = half_width + 1;

        let min_x = (x1.min(x2) - outer).max(0);
        let max_x = (x1.max(x2) + outer).min(width - 1);
        let min_y = (y1.min(y2) - outer).max(0);
        let max_y = (y1.max(y2) + outer).min(height - 1);

        // Bresenham's line algorithm to get centerline pixels
        let mut points = vec![];

        let mut x = x1;
        let mut y = y1;

        loop {
            points.push((x, y));

            if x == x2 && y == y2 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        // Rasterize thick line with anti-aliasing
        // No RGB565 conversions in primitives; rasterizer handles native formats.

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                // Find closest distance to centerline
                let mut min_dist2 = i32::MAX;

                for &(cx, cy) in &points {
                    let dx = px - cx;
                    let dy = py - cy;
                    let dist2 = dx * dx + dy * dy;
                    if dist2 < min_dist2 {
                        min_dist2 = dist2;
                    }
                }

                let r = half_width;
                let r2 = r * r;
                let r2_aa = (r + 1) * (r + 1);

                if min_dist2 > r2_aa {
                    continue;
                }

                let opa = if min_dist2 <= r2 {
                    255u8
                } else {
                    aa_coverage(min_dist2, r)
                };

                if opa == 0 {
                    continue;
                }

                let su = self.color.to_u32();
                let sr = ((su >> 24) & 0xFF) as u8;
                let sg = ((su >> 16) & 0xFF) as u8;
                let sb = ((su >> 8) & 0xFF) as u8;
                let sa = (su & 0xFF) as u8;
                let sa_eff = ((sa as u32 * self.alpha as u32) / 255) as u8;
                if sa_eff == 0 { continue; }
                let stroke_rgba = Rgba8888::rgba(sr, sg, sb, sa_eff);
                rasterizer.blend_pixel(px, py, stroke_rgba, opa);
            }
        }

        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}