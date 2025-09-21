#![no_std]

use crate::libs::gfx::two_d::types::{Point, Size, Rgba8888};
use crate::libs::gfx::two_d::raster::Rasterizer;
use crate::libs::gfx::two_d::gradients::{LinearGradient, RadialGradient};
use crate::libs::gfx::two_d::canvas2d::Canvas2D;

/// Paint system for fills - solid colors or gradients
#[derive(Clone, Debug)]
pub enum Paint {
    Solid(Rgba8888),
    Linear(LinearGradient),
    Radial(RadialGradient),
}

impl Paint {
    #[inline(always)]
    pub const fn solid(color: Rgba8888) -> Self {
        Self::Solid(color)
    }
    
    #[inline(always)]
    pub fn linear(start: Point, end: Point, start_color: Rgba8888, end_color: Rgba8888) -> Self {
        Self::Linear(LinearGradient::new(start, end, start_color, end_color))
    }
    
    #[inline(always)]
    pub fn radial(center: Point, radius: f32, inner_color: Rgba8888, outer_color: Rgba8888) -> Self {
        Self::Radial(RadialGradient::new(center, radius as u32, inner_color, outer_color))
    }
}

/// Stroke styling for outlines
#[derive(Clone, Debug)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f32,
    pub aa: bool,
}

impl Stroke {
    #[inline(always)]
    pub const fn new(color: Rgba8888, width: f32) -> Self {
        Self {
            paint: Paint::solid(color),
            width,
            aa: true,
        }
    }
    
    #[inline(always)]
    pub fn with_paint(paint: Paint, width: f32) -> Self {
        Self { paint, width, aa: true }
    }
    
    #[inline(always)]
    pub const fn no_aa(mut self) -> Self {
        self.aa = false;
        self
    }
}

/// Rectangle primitive with fluent API
#[derive(Clone, Debug)]
pub struct Rect {
    pos: Point,
    size: Size,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
    corner_radius: f32,
}

impl Rect {
    #[inline(always)]
    pub const fn new(pos: Point, size: Size) -> Self {
        Self {
            pos,
            size,
            fill: None,
            stroke: None,
            corner_radius: 0.0,
        }
    }
    
    #[inline(always)]
    pub fn fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }
    
    #[inline(always)]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    #[inline(always)]
    pub const fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }
    
    pub fn draw(self, canvas: &mut Canvas2D) {
        draw_rect_internal(canvas.raster_mut(), self.pos, self.size, self.fill, self.stroke, self.corner_radius);
    }
}

/// Circle primitive with fluent API
#[derive(Clone, Debug)]
pub struct Circle {
    center: Point,
    radius: f32,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
}

impl Circle {
    #[inline(always)]
    pub const fn new(center: Point, radius: f32) -> Self {
        Self {
            center,
            radius,
            fill: None,
            stroke: None,
        }
    }
    
    #[inline(always)]
    pub fn fill(mut self, paint: Paint) -> Self {
        self.fill = Some(paint);
        self
    }
    
    #[inline(always)]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    
    pub fn draw(self, canvas: &mut Canvas2D) {
        draw_circle_internal(canvas.raster_mut(), self.center, self.radius, self.fill, self.stroke);
    }
}

/// Line primitive with fluent API
#[derive(Clone, Debug)]
pub struct Line {
    start: Point,
    end: Point,
    stroke: Stroke,
}

impl Line {
    #[inline(always)]
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            stroke: Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0),
        }
    }
    
    #[inline(always)]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }
    
    pub fn draw(self, canvas: &mut Canvas2D) {
        draw_line_internal(canvas.raster_mut(), self.start, self.end, self.stroke);
    }
}

/// Arc primitive with fluent API
#[derive(Clone, Debug)]
pub struct Arc {
    center: Point,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: Stroke,
}

impl Arc {
    #[inline(always)]
    pub fn new(center: Point, radius: f32, start_angle: f32, end_angle: f32) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            stroke: Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0),
        }
    }
    
    #[inline(always)]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }
    
    pub fn draw(self, canvas: &mut Canvas2D) {
        draw_arc_internal(canvas.raster_mut(), self.center, self.radius, self.start_angle, self.end_angle, self.stroke);
    }
}

// ===== Internal drawing functions - single purpose, optimized =====

fn draw_rect_internal(raster: &mut dyn Rasterizer, pos: Point, size: Size, fill: Option<Paint>, stroke: Option<Stroke>, corner_radius: f32) {
    use crate::libs::gfx::two_d::primitives as prim;
    use crate::libs::gfx::two_d::types::Rect as GRect;
    
    let rect = GRect::new(pos, size);
    
    // Fill first
    if let Some(fill) = fill {
        match fill {
            Paint::Solid(color) => {
                if corner_radius > 0.0 {
                    prim::fill_rounded_rect(raster, rect, corner_radius as i32, color);
                } else {
                    prim::fill_rect(raster, rect, color);
                }
            }
            Paint::Linear(grad) => {
                if corner_radius > 0.0 {
                    prim::fill_rounded_rect_linear_gradient(raster, rect, corner_radius as i32, &grad);
                } else {
                    // Fallback to solid for now
                    prim::fill_rect(raster, rect, grad.start_color);
                }
            }
            Paint::Radial(_grad) => {
                // Fallback to solid for now
                prim::fill_rect(raster, rect, Rgba8888::opaque(255, 100, 100));
            }
        }
    }
    
    // Stroke second
    if let Some(stroke) = stroke {
        match stroke.paint {
            Paint::Solid(color) => {
                // Simple stroke outline - just draw border
                let thickness = stroke.width as i32;
                
                // Top edge
                let top_rect = crate::libs::gfx::two_d::types::Rect::new(
                    rect.top_left, 
                    Size::new(rect.size.width, thickness as u32)
                );
                prim::fill_rect(raster, top_rect, color);
                
                // Bottom edge
                let bottom_rect = crate::libs::gfx::two_d::types::Rect::new(
                    Point::new(rect.top_left.x, rect.top_left.y + rect.size.height as i32 - thickness), 
                    Size::new(rect.size.width, thickness as u32)
                );
                prim::fill_rect(raster, bottom_rect, color);
                
                // Left edge
                let left_rect = crate::libs::gfx::two_d::types::Rect::new(
                    rect.top_left, 
                    Size::new(thickness as u32, rect.size.height)
                );
                prim::fill_rect(raster, left_rect, color);
                
                // Right edge
                let right_rect = crate::libs::gfx::two_d::types::Rect::new(
                    Point::new(rect.top_left.x + rect.size.width as i32 - thickness, rect.top_left.y), 
                    Size::new(thickness as u32, rect.size.height)
                );
                prim::fill_rect(raster, right_rect, color);
            }
            _ => {
                // Gradient strokes - fallback to white outline
                let color = Rgba8888::opaque(255, 255, 255);
                let thickness = stroke.width as i32;
                
                let top_rect = crate::libs::gfx::two_d::types::Rect::new(
                    rect.top_left, 
                    Size::new(rect.size.width, thickness as u32)
                );
                prim::fill_rect(raster, top_rect, color);
            }
        }
    }
}

fn draw_circle_internal(raster: &mut dyn Rasterizer, center: Point, radius: f32, fill: Option<Paint>, stroke: Option<Stroke>) {
    use crate::libs::gfx::two_d::primitives as prim;
    
    // Fill first
    if let Some(fill) = fill {
        match fill {
            Paint::Solid(color) => {
                prim::fill_circle(raster, center, radius as i32, color);
            }
            Paint::Linear(_grad) => {
                // Fallback to solid
                prim::fill_circle(raster, center, radius as i32, Rgba8888::opaque(255, 100, 100));
            }
            Paint::Radial(_grad) => {
                // Fallback to solid
                prim::fill_circle(raster, center, radius as i32, Rgba8888::opaque(100, 255, 100));
            }
        }
    }
    
    // Stroke second
    if let Some(stroke) = stroke {
        match stroke.paint {
            Paint::Solid(color) => {
                if stroke.aa {
                    prim::draw_circle_aa(raster, center, radius as i32, color);
                } else {
                    // Fallback to AA version
                    prim::draw_circle_aa(raster, center, radius as i32, color);
                }
            }
            _ => {
                // Gradient strokes - fallback to solid
                let color = Rgba8888::opaque(255, 255, 255);
                prim::draw_circle_aa(raster, center, radius as i32, color);
            }
        }
    }
}

fn draw_line_internal(raster: &mut dyn Rasterizer, start: Point, end: Point, stroke: Stroke) {
    use crate::libs::gfx::two_d::primitives as prim;
    
    match stroke.paint {
        Paint::Solid(color) => {
            if stroke.aa {
                prim::draw_line_aa(raster, start, end, color);
            } else {
                // Fallback to AA version
                prim::draw_line_aa(raster, start, end, color);
            }
        }
        _ => {
            // Gradient lines - fallback to solid
            let color = Rgba8888::opaque(255, 255, 255);
            prim::draw_line_aa(raster, start, end, color);
        }
    }
}

fn draw_arc_internal(raster: &mut dyn Rasterizer, center: Point, radius: f32, start_angle: f32, end_angle: f32, stroke: Stroke) {
    use crate::libs::gfx::two_d::primitives as prim;
    
    match stroke.paint {
        Paint::Solid(color) => {
            if stroke.aa {
                prim::draw_arc_aa(raster, center, radius as i32, start_angle, end_angle, color);
            } else {
                prim::draw_arc(raster, center, radius as i32, start_angle, end_angle, color);
            }
        }
        _ => {
            // Gradient arcs - fallback to solid
            let color = Rgba8888::opaque(255, 255, 255);
            prim::draw_arc_aa(raster, center, radius as i32, start_angle, end_angle, color);
        }
    }
}
