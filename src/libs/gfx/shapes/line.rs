// file: src/shapes/line.rs
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
        if self.alpha == 0 { return; }
        if self.width <= 0 { return; }

        // Combine intrinsic color alpha with stroke alpha multiplier
        let cu = self.color.to_u32();
        let r = ((cu >> 24) & 0xFF) as u8;
        let g = ((cu >> 16) & 0xFF) as u8;
        let b = ((cu >> 8) & 0xFF) as u8;
        let a = (cu & 0xFF) as u8;
        let eff_a = ((a as u32 * self.alpha as u32) / 255) as u8;
        if eff_a == 0 { return; }
        let stroke_rgba = Rgba8888::rgba(r, g, b, eff_a);

        let x1 = self.x1;
        let y1 = self.y1;
        let x2 = self.x2;
        let y2 = self.y2;

        // Half widths to balance odd/even thickness
        let w = self.width - 1;
        let w_half0 = if w >= 0 { w >> 1 } else { 0 };
        let w_half1 = if w >= 0 { w_half0 + (w & 0x1) } else { 0 };

        // Axis-aligned fast paths
        if y1 == y2 {
            let xl = x1.min(x2);
            let xr = x1.max(x2);
            let yt = y1 - w_half0;
            let yb = y1 + w_half1;
            let w_rect = (xr - xl + 1).max(0);
            let h_rect = (yb - yt + 1).max(0);
            if w_rect > 0 && h_rect > 0 {
                rasterizer.fill_rect(xl, yt, w_rect, h_rect, stroke_rgba);
                let min_x = xl.max(0);
                let max_x = xr.min(rasterizer.width() as i32 - 1);
                let min_y = yt.max(0);
                let max_y = yb.min(rasterizer.height() as i32 - 1);
                rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
            }
            return;
        }

        if x1 == x2 {
            let yl = y1.min(y2);
            let yr = y1.max(y2);
            let xl = x1 - w_half0;
            let xr = x1 + w_half1;
            let w_rect = (xr - xl + 1).max(0);
            let h_rect = (yr - yl + 1).max(0);
            if w_rect > 0 && h_rect > 0 {
                rasterizer.fill_rect(xl, yl, w_rect, h_rect, stroke_rgba);
                let min_x = xl.max(0);
                let max_x = xr.min(rasterizer.width() as i32 - 1);
                let min_y = yl.max(0);
                let max_y = yr.min(rasterizer.height() as i32 - 1);
                rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
            }
            return;
        }

        // General case: Bresenham-like stepping with thickness
        let dx = x2 - x1;
        let dy = y2 - y1;
        let sx = if dx < 0 { -1 } else { 1 };
        let sy = if dy < 0 { -1 } else { 1 };
        let adx = dx.abs();
        let ady = dy.abs();

        let mut min_x = x1.min(x2) - w_half0;
        let mut max_x = x1.max(x2) + w_half1;
        let mut min_y = y1.min(y2) - w_half0;
        let mut max_y = y1.max(y2) + w_half1;

        if adx >= ady {
            let mut x = x1;
            let mut y = y1;
            let mut err = 0;
            for _ in 0..=adx {
                let y_start = y - w_half0;
                let len = self.width;
                rasterizer.blend_vspan_with(x, y_start, len, |_: usize| (stroke_rgba, 255));
                err += ady;
                if (err << 1) >= adx {
                    y += sy;
                    err -= adx;
                }
                x += sx;
            }
        } else {
            let mut x = x1;
            let mut y = y1;
            let mut err = 0;
            for _ in 0..=ady {
                let x_start = x - w_half0;
                let len = self.width;
                rasterizer.blend_hspan_with(x_start, y, len, |_: usize| (stroke_rgba, 255));
                err += adx;
                if (err << 1) >= ady {
                    x += sx;
                    err -= ady;
                }
                y += sy;
            }
        }

        min_x = min_x.max(0);
        min_y = min_y.max(0);
        max_x = max_x.min(rasterizer.width() as i32 - 1);
        max_y = max_y.min(rasterizer.height() as i32 - 1);
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}