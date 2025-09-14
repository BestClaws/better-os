use crate::libs::gfx::two_d::{Rasterizer, Rect, Point, Size, Rgba8888};
use crate::libs::gfx::two_d::draw::Draw;

use super::style::{Color, Fill, Stroke, CornerRadii};

/// Painter is a tiny convenience wrapper around a `Rasterizer` that provides
/// style-aware drawing helpers for widgets.
pub struct Painter<'a> {
    pub raster: &'a mut dyn Rasterizer,
}

impl<'a> Painter<'a> {
    pub fn new(raster: &'a mut dyn Rasterizer) -> Self {
        Self { raster }
    }

    #[inline(always)]
    fn to_rgba(color: Color) -> Rgba8888 {
        Rgba8888::opaque(color.r, color.g, color.b)
    }

    pub fn fill_rect(&mut self, rect: Rect, fill: Fill, corner: CornerRadii) {
        let bg = Self::to_rgba(fill.color);
        let mut d = Draw::new(self.raster);
        d.rect(rect)
            .fill_color(bg)
            .draw();
    }

    pub fn stroke_rect(&mut self, rect: Rect, stroke: Stroke, corner: CornerRadii) {
        if stroke.thickness == 0 { return; }
        let color = Self::to_rgba(stroke.color);
        let mut d = Draw::new(self.raster);
        d.rect(rect)
            .stroke(crate::libs::gfx::two_d::draw::stroke(stroke.thickness as i32, color))
            .draw();
    }

    pub fn rect(&mut self, rect: Rect, fill: Option<Fill>, stroke: Option<Stroke>, corner: CornerRadii) {
        if let Some(f) = fill { self.fill_rect(rect, f, corner); }
        if let Some(s) = stroke { self.stroke_rect(rect, s, corner); }
    }

    pub fn line(&mut self, p0: Point, p1: Point, stroke: Stroke) {
        let mut d = Draw::new(self.raster);
        d.line(p0, p1)
            .thickness(stroke.thickness as i32)
            .color(Self::to_rgba(stroke.color))
            .draw();
    }
}


