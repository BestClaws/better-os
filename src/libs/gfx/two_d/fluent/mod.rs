#![no_std]

// Embedded-graphics-inspired fluent API façade over our high-performance core

pub mod prelude {
    pub use super::{Drawable, DrawTarget};
    pub use super::{PrimitiveStyle, PrimitiveStyleBuilder};
    pub use super::{Rectangle, RoundedRectangle, Line, Arc};
    pub use super::{Text, MonoTextStyle};
}

use crate::libs::gfx::two_d::{Rasterizer, Point, Rect, Size, Rgb565, Rgba8888};
use crate::libs::gfx::two_d::draw::{StrokeStyle, FillStyle, CornerRadii, Draw as FluentDraw};
use crate::libs::gfx::two_d::primitives as prim;
use heapless::Vec;
use micromath::F32Ext;

// ===== DrawTarget adapter =====

pub trait DrawTarget {
    fn size(&self) -> Size;
    fn clear(&mut self, color: Rgb565);
    fn raster_mut(&mut self) -> &mut dyn Rasterizer;
}

impl<T: Rasterizer> DrawTarget for T {
    fn size(&self) -> Size { Size::new(self.width(), self.height()) }
    fn clear(&mut self, color: Rgb565) { Rasterizer::clear(self, color) }
    fn raster_mut(&mut self) -> &mut dyn Rasterizer { self }
}

impl DrawTarget for &mut dyn Rasterizer {
    fn size(&self) -> Size { Size::new((**self).width(), (**self).height()) }
    fn clear(&mut self, color: Rgb565) { (**self).clear(color) }
    fn raster_mut(&mut self) -> &mut dyn Rasterizer { *self }
}

impl DrawTarget for dyn Rasterizer {
    fn size(&self) -> Size { Size::new(self.width(), self.height()) }
    fn clear(&mut self, color: Rgb565) { Rasterizer::clear(self, color) }
    fn raster_mut(&mut self) -> &mut dyn Rasterizer { self }
}

// ===== Drawable trait =====

pub trait Drawable {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T);
}

// ===== Primitive styles =====

#[derive(Clone, Copy, Debug, Default)]
pub struct PrimitiveStyle {
    pub stroke: Option<StrokeStyle>,
    pub fill: Option<FillStyle>,
    pub stroke_rgba: Option<Rgba8888>,
}

pub struct PrimitiveStyleBuilder {
    style: PrimitiveStyle,
}

impl PrimitiveStyleBuilder {
    pub fn new() -> Self { Self { style: PrimitiveStyle::default() } }
    pub fn stroke_width(mut self, width: i32) -> Self {
        let color = self.style.stroke.map(|s| s.color).unwrap_or(Rgb565::WHITE);
        self.style.stroke = Some(StrokeStyle { color, thickness: width, aa: true });
        self
    }
    pub fn stroke_color(mut self, color: Rgb565) -> Self {
        let thickness = self.style.stroke.map(|s| s.thickness).unwrap_or(1);
        self.style.stroke = Some(StrokeStyle { color, thickness, aa: true });
        self
    }
    pub fn stroke_rgba(mut self, color: Rgba8888) -> Self {
        // Ensure there is a stroke style for thickness/aa management
        if self.style.stroke.is_none() {
            self.style.stroke = Some(StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa: true });
        }
        self.style.stroke_rgba = Some(color);
        self
    }
    pub fn aa(mut self, aa: bool) -> Self {
        if let Some(mut s) = self.style.stroke { s.aa = aa; self.style.stroke = Some(s); }
        else { self.style.stroke = Some(StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa }); }
        self
    }
    pub fn fill_color(mut self, color: Rgb565) -> Self {
        self.style.fill = Some(FillStyle::Solid(color));
        self
    }
    pub fn fill_rgba(mut self, color: Rgba8888) -> Self {
        self.style.fill = Some(FillStyle::Rgba(color));
        self
    }
    pub fn fill(mut self, fill: FillStyle) -> Self {
        self.style.fill = Some(fill);
        self
    }
    pub fn build(self) -> PrimitiveStyle { self.style }
}

// ===== Primitives =====

pub struct Rectangle {
    pub top_left: Point,
    pub size: Size,
    pub style: PrimitiveStyle,
}

impl Rectangle {
    pub fn new(top_left: Point, size: Size) -> Self {
        Self { top_left, size, style: PrimitiveStyle::default() }
    }
    pub fn into_styled(mut self, style: PrimitiveStyle) -> Self { self.style = style; self }
}

impl Drawable for Rectangle {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        let rect = Rect::new(self.top_left, self.size);
        let mut d = FluentDraw::new(target.raster_mut());
        let mut b = d.rect(rect);
        if let Some(fill) = self.style.fill { b = b.fill(fill); }
        if let Some(stroke) = self.style.stroke { b = b.stroke(stroke); }
        b.draw();
    }
}

pub struct RoundedRectangle {
    pub top_left: Point,
    pub size: Size,
    pub radii: CornerRadii,
    pub style: PrimitiveStyle,
}

impl RoundedRectangle {
    pub fn with_equal_corners(rect: Rect, r: i32) -> Self {
        Self { top_left: rect.top_left, size: rect.size, radii: CornerRadii::uniform(r), style: PrimitiveStyle::default() }
    }
    pub fn with_corners(rect: Rect, radii: CornerRadii) -> Self {
        Self { top_left: rect.top_left, size: rect.size, radii, style: PrimitiveStyle::default() }
    }
    pub fn into_styled(mut self, style: PrimitiveStyle) -> Self { self.style = style; self }
}

impl Drawable for RoundedRectangle {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        let rect = Rect::new(self.top_left, self.size);
        let mut d = FluentDraw::new(target.raster_mut());
        let mut b = d.rect(rect).corner_radii(self.radii);
        if let Some(fill) = self.style.fill { b = b.fill(fill); }
        if let Some(stroke) = self.style.stroke { b = b.stroke(stroke); }
        b.draw();
    }
}

pub struct Line {
    pub start: Point,
    pub end: Point,
    pub style: PrimitiveStyle,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Self { Self { start, end, style: PrimitiveStyle::default() } }
    pub fn into_styled(mut self, style: PrimitiveStyle) -> Self { self.style = style; self }
}

impl Drawable for Line {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        // Prefer RGBA stroke if specified
        if let Some(rgba) = self.style.stroke_rgba {
            prim::draw_line_rgba_aa(target.raster_mut(), self.start, self.end, rgba);
            return;
        }
        if let Some(stroke) = self.style.stroke {
            if stroke.thickness > 1 {
                prim::draw_line_thick_aa(target.raster_mut(), self.start, self.end, stroke.thickness, stroke.color);
            } else {
                prim::draw_line_aa(target.raster_mut(), self.start, self.end, stroke.color);
            }
        } else {
            prim::draw_line_aa(target.raster_mut(), self.start, self.end, Rgb565::WHITE);
        }
    }
}

pub struct Arc {
    pub center: Point,
    pub radius: i32,
    pub start: f32,
    pub end: f32,
    pub color: Rgb565,
}

impl Arc {
    pub fn new(center: Point, radius: i32, start: f32, end: f32) -> Self { Self { center, radius, start, end, color: Rgb565::WHITE } }
    pub fn stroke_color(mut self, color: Rgb565) -> Self { self.color = color; self }
}

impl Drawable for Arc {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        let mut d = FluentDraw::new(target.raster_mut());
        d.arc(self.center, self.radius, self.start, self.end).color(self.color).draw();
    }
}

// ===== Text bridge =====

pub struct MonoTextStyle {
    pub color: Rgb565,
}

impl MonoTextStyle {
    pub fn new(color: Rgb565) -> Self { Self { color } }
}

pub struct Text<'a> {
    pub position: Point,
    pub text: &'a str,
    pub style: MonoTextStyle,
}

impl<'a> Text<'a> {
    pub fn new(text: &'a str, position: Point, style: MonoTextStyle) -> Self { Self { position, text, style } }
}

impl<'a> Drawable for Text<'a> {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        // Bridge to existing TextRenderer with FONT_8X8
        let renderer = crate::libs::gfx::two_d::text::TextRenderer::new(&crate::libs::gfx::two_d::fonts::FONT_8X8)
            .with_color(self.style.color);
        renderer.draw_text(target.raster_mut(), self.position, self.text);
    }
}

// ===== Path (polyline and Bezier) =====

enum PathCmd {
    MoveTo(Point),
    LineTo(Point),
    QuadTo { c: Point, p: Point },
    CubicTo { c1: Point, c2: Point, p: Point },
    Close,
}

pub struct Path {
    cmds: Vec<PathCmd, 128>,
    style: PrimitiveStyle,
}

impl Path {
    pub fn new() -> Self { Self { cmds: Vec::new(), style: PrimitiveStyle::default() } }
    pub fn stroke_width(mut self, width: i32) -> Self {
        let color = self.style.stroke.map(|s| s.color).unwrap_or(Rgb565::WHITE);
        self.style.stroke = Some(StrokeStyle { color, thickness: width, aa: true });
        self
    }
    pub fn stroke_color(mut self, color: Rgb565) -> Self {
        let thickness = self.style.stroke.map(|s| s.thickness).unwrap_or(1);
        self.style.stroke = Some(StrokeStyle { color, thickness, aa: true });
        self
    }
    pub fn stroke_rgba(mut self, color: Rgba8888) -> Self {
        if self.style.stroke.is_none() {
            self.style.stroke = Some(StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa: true });
        }
        self.style.stroke_rgba = Some(color);
        self
    }
    pub fn aa(mut self, aa: bool) -> Self {
        if let Some(mut s) = self.style.stroke { s.aa = aa; self.style.stroke = Some(s); }
        else { self.style.stroke = Some(StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa }); }
        self
    }
    pub fn move_to(mut self, p: Point) -> Self { let _ = self.cmds.push(PathCmd::MoveTo(p)); self }
    pub fn line_to(mut self, p: Point) -> Self { let _ = self.cmds.push(PathCmd::LineTo(p)); self }
    pub fn quadratic_to(mut self, c: Point, p: Point) -> Self { let _ = self.cmds.push(PathCmd::QuadTo { c, p }); self }
    pub fn cubic_to(mut self, c1: Point, c2: Point, p: Point) -> Self { let _ = self.cmds.push(PathCmd::CubicTo { c1, c2, p }); self }
    pub fn close(mut self) -> Self { let _ = self.cmds.push(PathCmd::Close); self }
}

impl Drawable for Path {
    fn draw<T: DrawTarget + ?Sized>(self, target: &mut T) {
        let mut pen: Option<Point> = None;
        let mut start: Option<Point> = None;
        let r = target.raster_mut();
        let stroke = self.style.stroke.unwrap_or(StrokeStyle { color: Rgb565::WHITE, thickness: 1, aa: true });
        let rgba = self.style.stroke_rgba;

        for cmd in self.cmds.iter() {
            match *cmd {
                PathCmd::MoveTo(p) => { pen = Some(p); start = Some(p); }
                PathCmd::LineTo(p) => {
                    if let Some(prev) = pen {
                        if let Some(c) = rgba {
                            prim::draw_line_rgba_aa(r, prev, p, c);
                        } else if stroke.thickness > 1 {
                            prim::draw_line_thick_aa(r, prev, p, stroke.thickness, stroke.color);
                        } else {
                            prim::draw_line_aa(r, prev, p, stroke.color);
                        }
                        pen = Some(p);
                    } else { pen = Some(p); if start.is_none() { start = Some(p); } }
                }
                PathCmd::QuadTo { c, p } => {
                    if let Some(prev) = pen { approximate_quadratic(r, prev, c, p, stroke, rgba); pen = Some(p); }
                }
                PathCmd::CubicTo { c1, c2, p } => {
                    if let Some(prev) = pen { approximate_cubic(r, prev, c1, c2, p, stroke, rgba); pen = Some(p); }
                }
                PathCmd::Close => {
                    if let (Some(s), Some(cur)) = (start, pen) {
                        if cur != s {
                            if let Some(c) = rgba { prim::draw_line_rgba_aa(r, cur, s, c); }
                            else if stroke.thickness > 1 { prim::draw_line_thick_aa(r, cur, s, stroke.thickness, stroke.color); }
                            else { prim::draw_line_aa(r, cur, s, stroke.color); }
                        }
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

fn approximate_quadratic(
    r: &mut dyn Rasterizer,
    p0: Point,
    c: Point,
    p1: Point,
    style: StrokeStyle,
    rgba: Option<Rgba8888>,
) {
    let dx = (p1.x - p0.x) as f32; let dy = (p1.y - p0.y) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / 6.0).clamp(8.0, 64.0) as i32;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let it = 1.0 - t;
        let x = it * it * p0.x as f32 + 2.0 * it * t * c.x as f32 + t * t * p1.x as f32;
        let y = it * it * p0.y as f32 + 2.0 * it * t * c.y as f32 + t * t * p1.y as f32;
        let pt = Point::new(x.round() as i32, y.round() as i32);
        if let Some(col) = rgba { prim::draw_line_rgba_aa(r, prev, pt, col); }
        else if style.thickness <= 1 { prim::draw_line_aa(r, prev, pt, style.color); }
        else { prim::draw_line_thick_aa(r, prev, pt, style.thickness, style.color); }
        prev = pt;
    }
}

fn approximate_cubic(
    r: &mut dyn Rasterizer,
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    style: StrokeStyle,
    rgba: Option<Rgba8888>,
) {
    let dx = (p1.x - p0.x) as f32; let dy = (p1.y - p0.y) as f32;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len / 5.0).clamp(12.0, 96.0) as i32;
    let mut prev = p0;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let it = 1.0 - t;
        let x =
            it * it * it * p0.x as f32 +
            3.0 * it * it * t * c1.x as f32 +
            3.0 * it * t * t * c2.x as f32 +
            t * t * t * p1.x as f32;
        let y =
            it * it * it * p0.y as f32 +
            3.0 * it * it * t * c1.y as f32 +
            3.0 * it * t * t * c2.y as f32 +
            t * t * t * p1.y as f32;
        let pt = Point::new(x.round() as i32, y.round() as i32);
        if let Some(col) = rgba { prim::draw_line_rgba_aa(r, prev, pt, col); }
        else if style.thickness <= 1 { prim::draw_line_aa(r, prev, pt, style.color); }
        else { prim::draw_line_thick_aa(r, prev, pt, style.thickness, style.color); }
        prev = pt;
    }
}


