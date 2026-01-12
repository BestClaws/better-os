// file: src/shapes/arc.rs

use crate::color::Rgba8888;
use crate::shapes::circle::draw_circle_or_arc;
use crate::{Fill, Rasterizer, StrokeStyle};

pub struct Arc {
    cx: i32,
    cy: i32,
    radius: i32,
    start_deg: i32,
    end_deg: i32,
    stroke: StrokeStyle,
    fill: Option<Fill>,
}

impl Arc {
    pub fn new(cx: i32, cy: i32, radius: i32, start_deg: i32, end_deg: i32) -> Self {
        Self {
            cx,
            cy,
            radius,
            start_deg,
            end_deg,
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

impl super::Shape for Arc {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        draw_circle_or_arc(
            rasterizer,
            self.cx,
            self.cy,
            self.radius,
            self.stroke,
            self.fill,
            Some((self.start_deg, self.end_deg)),
        );
    }
}
