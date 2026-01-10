use crate::libs::gfx::core::color::{Color, ColorAlpha, Opacity};
use crate::libs::gfx::core::blend::{BlendDescriptor, BlendSource, BlendMode};
use crate::libs::gfx::core::geometry::{PointF, Rect};
use crate::libs::gfx::draw_target::DrawTarget;
use crate::libs::gfx::layer::Layer;

/// Triangle primitive builder
pub struct Triangle<'a> {
    layer: &'a mut Layer<'a>,
    points: [PointF; 3],
    color: Color,
    opacity: Opacity,
}

impl<'a> Triangle<'a> {
    pub fn new(layer: &'a mut Layer<'a>, p1: PointF, p2: PointF, p3: PointF) -> Self {
        Self {
            layer,
            points: [p1, p2, p3],
            color: Color::WHITE,
            opacity: Opacity::OPAQUE,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn points(mut self, p1: PointF, p2: PointF, p3: PointF) -> Self {
        self.points = [p1, p2, p3];
        self
    }

    pub fn draw(mut self) {
        // Sort vertices by y coordinate
        let mut pts = self.points;
        if pts[0].y > pts[1].y {
            pts.swap(0, 1);
        }
        if pts[1].y > pts[2].y {
            pts.swap(1, 2);
        }
        if pts[0].y > pts[1].y {
            pts.swap(0, 1);
        }

        // Rasterize triangle using scanline algorithm
        self.fill_triangle(pts[0], pts[1], pts[2]);
    }

    fn fill_triangle(&mut self, v0: PointF, v1: PointF, v2: PointF) {
        let y0 = v0.y as i32;
        let y1 = v1.y as i32;
        let y2 = v2.y as i32;

        // Skip degenerate triangles
        if y0 == y2 {
            return;
        }

        // Compute slopes
        let inv_slope_1 = if y1 != y0 {
            (v1.x - v0.x) / (v1.y - v0.y)
        } else {
            0.0
        };

        let inv_slope_2 = if y2 != y0 {
            (v2.x - v0.x) / (v2.y - v0.y)
        } else {
            0.0
        };

        let inv_slope_3 = if y2 != y1 {
            (v2.x - v1.x) / (v2.y - v1.y)
        } else {
            0.0
        };

        // Fill bottom-flat triangle
        if y1 == y2 {
            self.fill_bottom_flat_triangle(v0, v1, v2);
        }
        // Fill top-flat triangle
        else if y0 == y1 {
            self.fill_top_flat_triangle(v0, v1, v2);
        }
        // General case: split into two triangles
        else {
            // Compute split point
            let x_split = v0.x + ((v1.y - v0.y) / (v2.y - v0.y)) * (v2.x - v0.x);
            let v_split = PointF::new(x_split, v1.y);

            self.fill_bottom_flat_triangle(v0, v1, v_split);
            self.fill_top_flat_triangle(v1, v_split, v2);
        }
    }

    fn fill_bottom_flat_triangle(&mut self, v0: PointF, v1: PointF, v2: PointF) {
        let inv_slope1 = (v1.x - v0.x) / (v1.y - v0.y);
        let inv_slope2 = (v2.x - v0.x) / (v2.y - v0.y);

        let mut cur_x1 = v0.x;
        let mut cur_x2 = v0.x;

        let y_start = v0.y as i32;
        let y_end = v1.y as i32;

        for y in y_start..=y_end {
            self.draw_scanline(y, cur_x1 as i32, cur_x2 as i32);
            cur_x1 += inv_slope1;
            cur_x2 += inv_slope2;
        }
    }

    fn fill_top_flat_triangle(&mut self, v0: PointF, v1: PointF, v2: PointF) {
        let inv_slope1 = (v2.x - v0.x) / (v2.y - v0.y);
        let inv_slope2 = (v2.x - v1.x) / (v2.y - v1.y);

        let mut cur_x1 = v2.x;
        let mut cur_x2 = v2.x;

        let y_start = v2.y as i32;
        let y_end = v0.y as i32;

        for y in (y_end..=y_start).rev() {
            self.draw_scanline(y, cur_x1 as i32, cur_x2 as i32);
            cur_x1 -= inv_slope1;
            cur_x2 -= inv_slope2;
        }
    }

    fn draw_scanline(&mut self, y: i32, mut x1: i32, mut x2: i32) {
        if x1 > x2 {
            core::mem::swap(&mut x1, &mut x2);
        }

        if x2 < x1 {
            return;
        }

        let rect = Rect::new(x1, y, (x2 - x1 + 1) as u32, 1);
        let desc = BlendDescriptor {
            mode: BlendMode::Normal,
            source: Some(BlendSource::SolidColor { color: ColorAlpha::from_rgb(self.color, self.opacity) }),
            dest_rect: rect,
            opacity: self.opacity,
            mask: None,
        };
        self.layer.blend(&desc);
    }
}

impl<'a> Layer<'a> {
    pub fn triangle(&'a mut self, p1: PointF, p2: PointF, p3: PointF) -> Triangle<'a> {
        Triangle::new(self, p1, p2, p3)
    }
}
