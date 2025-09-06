use crate::libs::gfx::two_d::{Rasterizer, Rect, Point, Size, Rgb565};
use crate::libs::gfx::two_d::{fill_rect_styled, draw_line_thick_aa, draw_rect_outline_aa, fill_rect, fill_rounded_rect};

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
    fn to_rgb565(color: Color) -> Rgb565 {
        Rgb565::from_rgb(color.r, color.g, color.b)
    }

    pub fn fill_rect(&mut self, rect: Rect, fill: Fill, corner: CornerRadii) {
        let bg = Self::to_rgb565(fill.color);
        match corner.uniform {
            0 => fill_rect(self.raster, rect, bg),
            r => fill_rounded_rect(self.raster, rect, r as i32, bg),
        }
    }

    pub fn stroke_rect(&mut self, rect: Rect, stroke: Stroke, corner: CornerRadii) {
        if stroke.thickness == 0 { return; }
        let color = Self::to_rgb565(stroke.color);
        if corner.uniform == 0 {
            draw_rect_outline_aa(self.raster, rect, stroke.thickness as i32, color);
        } else {
            // Use styled helper
            fill_rect_styled(
                self.raster,
                rect,
                corner.uniform as i32,
                Rgb565::from_rgb(0, 0, 0), // background not used, border only
                Some(color),
                stroke.thickness as i32,
            );
        }
    }

    pub fn rect(&mut self, rect: Rect, fill: Option<Fill>, stroke: Option<Stroke>, corner: CornerRadii) {
        if let Some(f) = fill { self.fill_rect(rect, f, corner); }
        if let Some(s) = stroke { self.stroke_rect(rect, s, corner); }
    }

    pub fn line(&mut self, p0: Point, p1: Point, stroke: Stroke) {
        draw_line_thick_aa(self.raster, p0, p1, stroke.thickness as i32, Self::to_rgb565(stroke.color));
    }
}


