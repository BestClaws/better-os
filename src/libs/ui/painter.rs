use crate::libs::gfx::two_d::{Rasterizer, Rect, Point, Size, Rgba8888};
use crate::libs::gfx::two_d::{Canvas2D, FluentRect, Line, Paint, Stroke};

use super::style::{Color, Fill, Stroke as StyleStroke, CornerRadii};

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
        let mut canvas = Canvas2D::new(self.raster);
        FluentRect::new(rect.top_left, rect.size)
            .fill(Paint::solid(bg))
            .draw(&mut canvas);
    }

    pub fn stroke_rect(&mut self, rect: Rect, stroke: StyleStroke, corner: CornerRadii) {
        if stroke.thickness == 0 { return; }
        let color = Self::to_rgba(stroke.color);
        let mut canvas = Canvas2D::new(self.raster);
        FluentRect::new(rect.top_left, rect.size)
            .stroke(Stroke::new(color, stroke.thickness as f32))
            .draw(&mut canvas);
    }

    pub fn rect(&mut self, rect: Rect, fill: Option<Fill>, stroke: Option<StyleStroke>, corner: CornerRadii) {
        if let Some(f) = fill { self.fill_rect(rect, f, corner); }
        if let Some(s) = stroke { self.stroke_rect(rect, s, corner); }
    }

    pub fn line(&mut self, p0: Point, p1: Point, stroke: StyleStroke) {
        let mut canvas = Canvas2D::new(self.raster);
        Line::new(p0, p1)
            .stroke(Stroke::new(Self::to_rgba(stroke.color), stroke.thickness as f32))
            .draw(&mut canvas);
    }
}


