// file: src/shapes/arc.rs

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{rgba8888_to_rgb565_and_alpha, Fill, Rasterizer};
use crate::libs::gfx::shapes::circle::draw_circle_or_arc;

pub struct Arc {
    cx: i32,
    cy: i32,
    radius: i32,
    start_deg: i32,
    end_deg: i32,
    stroke_width: i32,
    stroke_color: u16,
    stroke_alpha: u8,
    fill: Option<Fill>,
}

impl Arc {
    pub fn new(
        cx: i32,
        cy: i32,
        radius: i32,
        start_deg: i32,
        end_deg: i32,
    ) -> Self {
        Self {
            cx,
            cy,
            radius,
            start_deg,
            end_deg,
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

impl super::Shape for Arc {
    fn draw<R: Rasterizer>(&self, rasterizer: &mut R) {
        draw_circle_or_arc(rasterizer, self.cx, self.cy, self.radius, self.stroke_width, self.stroke_color, self.stroke_alpha, self.fill, Some((self.start_deg, self.end_deg)));
    }
}