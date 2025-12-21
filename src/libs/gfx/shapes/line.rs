// file: src/shapes/line.rs
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::Rasterizer;
use libm::{fabsf, floorf};

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

        // General case
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

        // If sloped line, use AA. For width==1 use Wu directly; for thick lines draw AA edges + solid core.
        if adx != 0 && ady != 0 {
            // Helpers
            #[inline(always)]
            fn rfpart(x: f32) -> f32 { 1.0 - (x - floorf(x)) }
            #[inline(always)]
            fn fpart(x: f32) -> f32 { x - floorf(x) }

            let mut x0 = x1 as f32;
            let mut y0 = y1 as f32;
            let mut x1f = x2 as f32;
            let mut y1f = y2 as f32;

            let steep = ady > adx;

            // Swap for steep
            if steep {
                core::mem::swap(&mut x0, &mut y0);
                core::mem::swap(&mut x1f, &mut y1f);
            }
            // Ensure x0 <= x1
            if x0 > x1f {
                core::mem::swap(&mut x0, &mut x1f);
                core::mem::swap(&mut y0, &mut y1f);
            }

            let dx_f = x1f - x0;
            let dy_f = y1f - y0;
            let gradient = if dx_f == 0.0 { 1.0 } else { dy_f / dx_f };
            // Render depending on width
            if self.width == 1 {
                // First endpoint
                let xend = floorf(x0 + 0.5);
                let yend = y0 + gradient * (xend - x0);
                let xgap = rfpart(x0 + 0.5);
                let xpxl1 = xend as i32; // first pixel x
                let ypxl1 = floorf(yend) as i32;

                let c1 = (rfpart(yend) * xgap * 255.0) as i32;
                let c2 = (fpart(yend) * xgap * 255.0) as i32;
                if steep {
                    rasterizer.blend_pixel(ypxl1, xpxl1, stroke_rgba, c1.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(ypxl1 + 1, xpxl1, stroke_rgba, c2.clamp(0, 255) as u8);
                } else {
                    rasterizer.blend_pixel(xpxl1, ypxl1, stroke_rgba, c1.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl1, ypxl1 + 1, stroke_rgba, c2.clamp(0, 255) as u8);
                }
                let mut intery = yend + gradient; // first y-intersection for the main loop

                // Second endpoint
                let xend2 = floorf(x1f + 0.5);
                let yend2 = y1f + gradient * (xend2 - x1f);
                let xgap2 = fpart(x1f + 0.5);
                let xpxl2 = xend2 as i32; // last pixel x
                let ypxl2 = floorf(yend2) as i32;

                // Main loop
                for x in (xpxl1 + 1)..(xpxl2) {
                    let ipart_y = floorf(intery) as i32;
                    let c_top = (rfpart(intery) * 255.0) as i32;
                    let c_bot = (fpart(intery) * 255.0) as i32;
                    if steep {
                        rasterizer.blend_pixel(ipart_y, x, stroke_rgba, c_top.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(ipart_y + 1, x, stroke_rgba, c_bot.clamp(0, 255) as u8);
                    } else {
                        rasterizer.blend_pixel(x, ipart_y, stroke_rgba, c_top.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(x, ipart_y + 1, stroke_rgba, c_bot.clamp(0, 255) as u8);
                    }
                    intery += gradient;
                }

                // Draw the second endpoint pixels
                let c1e = (rfpart(yend2) * xgap2 * 255.0) as i32;
                let c2e = (fpart(yend2) * xgap2 * 255.0) as i32;
                if steep {
                    rasterizer.blend_pixel(ypxl2, xpxl2, stroke_rgba, c1e.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(ypxl2 + 1, xpxl2, stroke_rgba, c2e.clamp(0, 255) as u8);
                } else {
                    rasterizer.blend_pixel(xpxl2, ypxl2, stroke_rgba, c1e.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl2, ypxl2 + 1, stroke_rgba, c2e.clamp(0, 255) as u8);
                }
            } else {
                // Thick AA line: draw AA top/bottom edges and fill the interior
                let half = (self.width as f32) / 2.0;

                // First endpoint setup
                let xend = floorf(x0 + 0.5);
                let yend = y0 + gradient * (xend - x0);
                let mut intery = yend + gradient; // for core fill

                let xend2 = floorf(x1f + 0.5);
                let yend2 = y1f + gradient * (xend2 - x1f);
                let xpxl1 = xend as i32;
                let xpxl2 = xend2 as i32;

                // Draw start cap edges
                let y_top = yend - half;
                let y_bot = yend + half;
                let y_top_i = floorf(y_top) as i32;
                let y_bot_i = floorf(y_bot) as i32;
                let cov_top0 = (rfpart(y_top) * 255.0) as i32;
                let cov_top1 = (fpart(y_top) * 255.0) as i32;
                let cov_bot0 = (rfpart(y_bot) * 255.0) as i32;
                let cov_bot1 = (fpart(y_bot) * 255.0) as i32;
                if steep {
                    rasterizer.blend_pixel(y_top_i, xpxl1, stroke_rgba, cov_top0.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_top_i + 1, xpxl1, stroke_rgba, cov_top1.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_bot_i, xpxl1, stroke_rgba, cov_bot0.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_bot_i + 1, xpxl1, stroke_rgba, cov_bot1.clamp(0, 255) as u8);
                    // Core between edges (steep: device y = xpxl1, span along device x)
                    let x_core_start = y_top_i + 1;
                    let x_core_end = y_bot_i;
                    if x_core_end >= x_core_start {
                        rasterizer.blend_hspan_with(x_core_start, xpxl1, x_core_end - x_core_start + 1, |_: usize| (stroke_rgba, 255));
                    }
                } else {
                    rasterizer.blend_pixel(xpxl1, y_top_i, stroke_rgba, cov_top0.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl1, y_top_i + 1, stroke_rgba, cov_top1.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl1, y_bot_i, stroke_rgba, cov_bot0.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl1, y_bot_i + 1, stroke_rgba, cov_bot1.clamp(0, 255) as u8);
                    // Core between edges
                    let y_core_start = y_top_i + 1;
                    let y_core_end = y_bot_i;
                    if y_core_end >= y_core_start {
                        rasterizer.blend_vspan_with(xpxl1, y_core_start, y_core_end - y_core_start + 1, |_: usize| (stroke_rgba, 255));
                    }
                }

                // Main loop across x
                for x in (xpxl1 + 1)..(xpxl2) {
                    let y_center = intery;
                    let y_top = y_center - half;
                    let y_bot = y_center + half;
                    let y_top_i = floorf(y_top) as i32;
                    let y_bot_i = floorf(y_bot) as i32;

                    // Edge coverages
                    let cov_top0 = (rfpart(y_top) * 255.0) as i32;
                    let cov_top1 = (fpart(y_top) * 255.0) as i32;
                    let cov_bot0 = (rfpart(y_bot) * 255.0) as i32;
                    let cov_bot1 = (fpart(y_bot) * 255.0) as i32;

                    if steep {
                        rasterizer.blend_pixel(y_top_i, x, stroke_rgba, cov_top0.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(y_top_i + 1, x, stroke_rgba, cov_top1.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(y_bot_i, x, stroke_rgba, cov_bot0.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(y_bot_i + 1, x, stroke_rgba, cov_bot1.clamp(0, 255) as u8);

                        let x_core_start = y_top_i + 1;
                        let x_core_end = y_bot_i;
                        if x_core_end >= x_core_start {
                            rasterizer.blend_hspan_with(x_core_start, x, x_core_end - x_core_start + 1, |_: usize| (stroke_rgba, 255));
                        }
                    } else {
                        rasterizer.blend_pixel(x, y_top_i, stroke_rgba, cov_top0.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(x, y_top_i + 1, stroke_rgba, cov_top1.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(x, y_bot_i, stroke_rgba, cov_bot0.clamp(0, 255) as u8);
                        rasterizer.blend_pixel(x, y_bot_i + 1, stroke_rgba, cov_bot1.clamp(0, 255) as u8);

                        let y_core_start = y_top_i + 1;
                        let y_core_end = y_bot_i;
                        if y_core_end >= y_core_start {
                            rasterizer.blend_vspan_with(x, y_core_start, y_core_end - y_core_start + 1, |_: usize| (stroke_rgba, 255));
                        }
                    }

                    intery += gradient;
                }

                // End cap edges
                let y_top2 = yend2 - half;
                let y_bot2 = yend2 + half;
                let y_top2_i = floorf(y_top2) as i32;
                let y_bot2_i = floorf(y_bot2) as i32;
                let cov_top20 = (rfpart(y_top2) * 255.0) as i32;
                let cov_top21 = (fpart(y_top2) * 255.0) as i32;
                let cov_bot20 = (rfpart(y_bot2) * 255.0) as i32;
                let cov_bot21 = (fpart(y_bot2) * 255.0) as i32;
                if steep {
                    rasterizer.blend_pixel(y_top2_i, xpxl2, stroke_rgba, cov_top20.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_top2_i + 1, xpxl2, stroke_rgba, cov_top21.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_bot2_i, xpxl2, stroke_rgba, cov_bot20.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(y_bot2_i + 1, xpxl2, stroke_rgba, cov_bot21.clamp(0, 255) as u8);
                    let x_core_start = y_top2_i + 1;
                    let x_core_end = y_bot2_i;
                    if x_core_end >= x_core_start {
                        rasterizer.blend_hspan_with(x_core_start, xpxl2, x_core_end - x_core_start + 1, |_: usize| (stroke_rgba, 255));
                    }
                } else {
                    rasterizer.blend_pixel(xpxl2, y_top2_i, stroke_rgba, cov_top20.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl2, y_top2_i + 1, stroke_rgba, cov_top21.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl2, y_bot2_i, stroke_rgba, cov_bot20.clamp(0, 255) as u8);
                    rasterizer.blend_pixel(xpxl2, y_bot2_i + 1, stroke_rgba, cov_bot21.clamp(0, 255) as u8);
                    let y_core_start = y_top2_i + 1;
                    let y_core_end = y_bot2_i;
                    if y_core_end >= y_core_start {
                        rasterizer.blend_vspan_with(xpxl2, y_core_start, y_core_end - y_core_start + 1, |_: usize| (stroke_rgba, 255));
                    }
                }
            }
        } else {
            // Non-AA fallback for thick lines (and remaining cases)
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
        }

        min_x = min_x.max(0);
        min_y = min_y.max(0);
        max_x = max_x.min(rasterizer.width() as i32 - 1);
        max_y = max_y.min(rasterizer.height() as i32 - 1);
        rasterizer.mark_dirty(min_x, min_y, max_x, max_y);
    }
}