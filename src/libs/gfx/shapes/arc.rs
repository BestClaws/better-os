// file: src/shapes/arc.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::shapes::circle::draw_circle_or_arc;
use crate::libs::gfx::{Fill, Rasterizer};

pub struct Arc {
    cx: i32,
    cy: i32,
    radius: i32,
    start_deg: i32,
    end_deg: i32,
    stroke_width: i32,
    stroke_color: Rgba8888,
    stroke_alpha: u8,
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

impl super::Shape for Arc {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        draw_circle_or_arc(
            rasterizer,
            self.cx,
            self.cy,
            self.radius,
            self.stroke_width,
            self.stroke_color,
            self.stroke_alpha,
            self.fill,
            Some((self.start_deg, self.end_deg)),
        );
    }
}
