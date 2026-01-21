//! Fluent primitive builders mapped onto the legacy draw routines.

extern crate alloc;

use alloc::{string::String, vec::Vec};

use crate::color::Rgba8888;
use crate::primitives::arc::{draw_arc, ArcDsc};
use crate::primitives::blur::{draw_blur, BlurDsc};
use crate::primitives::common::geometry::clip_to_raster;
use crate::primitives::label::{
    draw_label, line_height_for_font, measure_text_with_font, FontId, LabelDsc, TextDecor,
};
use crate::primitives::line::{draw_line, LineDsc};
use crate::primitives::rectangle::{draw_rect, RectDsc};
use crate::primitives::triangle::{draw_triangle, TriangleDsc};
use crate::primitives::vector::{
    draw_vector, ColorStop as VectorColorStop, FPoint, FillRule as VectorFillRule,
    LinearGradient as VectorLinearGradient, StrokeCap as VectorStrokeCap,
    StrokeJoin as VectorStrokeJoin, VectorDsc, VectorFill, VectorPath, VectorStroke,
};
use crate::types::{self, Area, Opa, Point, OPA_COVER, RADIUS_CIRCLE};
use crate::Rasterizer;

use micromath::F32Ext;

use super::styles::{
    Axis, FillPlan, GradientKind, GradientPlan, OutlinePlan, ShadowPlan, StrokeJoinStyle,
    StrokePlan,
};
use super::types::{
    Angle, Angles, DashPattern, FontHandle, LabelAlignment, LabelContent, LabelContentPlan,
    LabelDecor, LabelOpacity, LabelSpacing, LineCap, LineCaps, PathPlan, Radius, Vertices,
    WindingPlan,
};

struct FluentClipper<'a, R: Rasterizer> {
    inner: &'a mut R,
    clip: Area,
}

impl<'a, R: Rasterizer> FluentClipper<'a, R> {
    fn new(inner: &'a mut R, clip: Area) -> Self {
        Self { inner, clip }
    }
}

impl<'a, R: Rasterizer> Rasterizer for FluentClipper<'a, R> {
    fn width(&self) -> usize {
        self.inner.width()
    }

    fn height(&self) -> usize {
        self.inner.height()
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.inner.buffer_mut()
    }

    fn clear(&mut self, color: Rgba8888) {
        self.inner.clear(color);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        if coverage == 0 {
            return;
        }
        let clip = self.clip;
        if x < clip.x1 || x > clip.x2 || y < clip.y1 || y > clip.y2 {
            return;
        }
        self.inner.blend_pixel(x, y, color, coverage);
    }

    fn blend_hspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        if len <= 0 {
            return;
        }
        let clip = self.clip;
        if y < clip.y1 || y > clip.y2 {
            return;
        }
        let count = len as usize;
        let start = x;
        let end = start + len - 1;
        let clip_start = clip.x1.max(start);
        let clip_end = clip.x2.min(end);
        if clip_start > clip_end {
            return;
        }

        for offset in 0..count {
            let px = start + offset as i32;
            if px < clip_start || px > clip_end {
                continue;
            }
            let (color, coverage) = f(offset);
            if coverage == 0 {
                continue;
            }
            self.inner.blend_pixel(px, y, color, coverage);
        }
    }

    fn blend_vspan_with(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        mut f: impl FnMut(usize) -> (Rgba8888, u8),
    ) {
        if len <= 0 {
            return;
        }
        let clip = self.clip;
        if x < clip.x1 || x > clip.x2 {
            return;
        }
        let count = len as usize;
        let start = y;
        let end = start + len - 1;
        let clip_start = clip.y1.max(start);
        let clip_end = clip.y2.min(end);
        if clip_start > clip_end {
            return;
        }

        for offset in 0..count {
            let py = start + offset as i32;
            if py < clip_start || py > clip_end {
                continue;
            }
            let (color, coverage) = f(offset);
            if coverage == 0 {
                continue;
            }
            self.inner.blend_pixel(x, py, color, coverage);
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        if w <= 0 || h <= 0 {
            return;
        }
        let clip = self.clip;
        let area = Area::new(x, y, x + w - 1, y + h - 1);
        if let Some(int) = area.intersect(&clip) {
            self.inner
                .fill_rect(int.x1, int.y1, int.width(), int.height(), color);
        }
    }

    fn stamp_rgb_zero_alpha(&mut self, x: i32, y: i32, color: Rgba8888) {
        let clip = self.clip;
        if x < clip.x1 || x > clip.x2 || y < clip.y1 || y > clip.y2 {
            return;
        }
        self.inner.stamp_rgb_zero_alpha(x, y, color);
    }

    unsafe fn unsafe_buffer_mut(&mut self) -> &mut [u8] {
        self.inner.unsafe_buffer_mut()
    }
}

// -------------------------------------------------------------------------------------------------
// Rectangle

pub struct Rect;

impl Rect {
    pub fn new() -> RectBuilder {
        RectBuilder::default()
    }
}

#[derive(Default)]
pub struct RectBuilder {
    area: Option<Area>,
    radius: Option<Radius>,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    outline: Option<OutlinePlan>,
    shadow: Option<ShadowPlan>,
    clip: Option<Area>,
}

impl RectBuilder {
    pub fn area(mut self, area: Area) -> Self {
        self.area = Some(area);
        self
    }

    pub fn radius(mut self, radius: Radius) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn fill(mut self, fill: FillPlan) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn outline(mut self, outline: OutlinePlan) -> Self {
        self.outline = Some(outline);
        self
    }

    pub fn shadow(mut self, shadow: ShadowPlan) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> RectPrimitive {
        let area = self
            .area
            .expect("Rect::area(Area) must be specified before finish()");

        RectPrimitive {
            area,
            radius: self.radius.unwrap_or(Radius::uniform(0)),
            stroke: self.stroke,
            fill: self.fill,
            outline: self.outline,
            shadow: self.shadow,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RectPrimitive {
    area: Area,
    radius: Radius,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    outline: Option<OutlinePlan>,
    shadow: Option<ShadowPlan>,
    clip: Option<Area>,
}

impl RectPrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        let mut dsc = RectDsc::new();
        apply_radius(&mut dsc, self.radius);
        apply_fill(&mut dsc, self.fill.as_ref());
        apply_stroke(&mut dsc, self.stroke.as_ref());
        apply_outline(&mut dsc, self.outline.as_ref());
        apply_shadow(&mut dsc, self.shadow.as_ref());

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_rect(&mut clipped, &dsc, &self.area, Some(clamped));
            }
        } else {
            draw_rect(rast, &dsc, &self.area, None);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Circle

pub struct Circle;

impl Circle {
    pub fn new() -> CircleBuilder {
        CircleBuilder::default()
    }
}

#[derive(Default)]
pub struct CircleBuilder {
    center: Option<Point>,
    radius: Option<i32>,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    shadow: Option<ShadowPlan>,
    clip: Option<Area>,
}

impl CircleBuilder {
    pub fn center(mut self, center: Point) -> Self {
        self.center = Some(center);
        self
    }

    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = Some(radius.max(0));
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn fill(mut self, fill: FillPlan) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn shadow(mut self, shadow: ShadowPlan) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> CirclePrimitive {
        let center = self
            .center
            .expect("Circle::center(Point) must be specified before finish()");
        let radius = self
            .radius
            .expect("Circle::radius(i32) must be specified before finish()");

        CirclePrimitive {
            center,
            radius,
            stroke: self.stroke,
            fill: self.fill,
            shadow: self.shadow,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CirclePrimitive {
    center: Point,
    radius: i32,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    shadow: Option<ShadowPlan>,
    clip: Option<Area>,
}

impl CirclePrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        if self.radius <= 0 {
            return;
        }

        let area = Area::new(
            self.center.x - self.radius,
            self.center.y - self.radius,
            self.center.x + self.radius - 1,
            self.center.y + self.radius - 1,
        );

        let mut dsc = RectDsc::new();
        dsc.radius = RADIUS_CIRCLE;
        apply_fill(&mut dsc, self.fill.as_ref());
        apply_stroke(&mut dsc, self.stroke.as_ref());
        apply_shadow(&mut dsc, self.shadow.as_ref());

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_rect(&mut clipped, &dsc, &area, Some(clamped));
            }
        } else {
            draw_rect(rast, &dsc, &area, None);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Arc

pub struct Arc;

impl Arc {
    pub fn new() -> ArcBuilder {
        ArcBuilder::default()
    }
}

#[derive(Default)]
pub struct ArcBuilder {
    center: Option<Point>,
    radius: Option<Radius>,
    angles: Option<Angles>,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
    rounded: bool,
}

impl ArcBuilder {
    pub fn center(mut self, center: Point) -> Self {
        self.center = Some(center);
        self
    }

    pub fn radius(mut self, radius: Radius) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn angles(mut self, angles: Angles) -> Self {
        self.angles = Some(angles);
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn fill(mut self, fill: FillPlan) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    pub fn finish(self) -> ArcPrimitive {
        let center = self
            .center
            .expect("Arc::center(Point) must be specified before finish()");
        let radius = self
            .radius
            .expect("Arc::radius(Radius::ring) must be specified before finish()");
        let angles = self
            .angles
            .expect("Arc::angles(Angles) must be specified before finish()");

        ArcPrimitive {
            center,
            radius,
            angles,
            stroke: self.stroke,
            fill: self.fill,
            clip: self.clip,
            rounded: self.rounded,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArcPrimitive {
    center: Point,
    radius: Radius,
    angles: Angles,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
    rounded: bool,
}

impl ArcPrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        let outer_radius = match self.radius {
            Radius::Ring { outer, .. } => outer,
            Radius::Uniform(value) => value,
            Radius::Corners { .. } => 0,
        };
        if outer_radius <= 0 {
            return;
        }

        let mut dsc = ArcDsc::new(
            self.center,
            outer_radius,
            self.angles.start().as_degrees(),
            self.angles.end().as_degrees(),
        );
        dsc.rounded = self.rounded;

        if let Some(stroke) = &self.stroke {
            dsc.width = if stroke.width > 0 {
                stroke.width
            } else {
                match self.radius {
                    Radius::Ring { inner, .. } => (outer_radius - inner).max(1),
                    _ => 1,
                }
            };
            dsc.color = stroke
                .color
                .unwrap_or_else(|| fallback_gradient_color(stroke.gradient.as_ref()));
            dsc.opa = stroke.opacity;
        } else {
            dsc.width = match self.radius {
                Radius::Ring { inner, .. } => (outer_radius - inner).max(1),
                _ => 1,
            };
            dsc.color = Rgba8888::WHITE;
            dsc.opa = OPA_COVER;
        }

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_arc(&mut clipped, &dsc);
            }
        } else {
            draw_arc(rast, &dsc);
        }

        if self.fill.is_some() {
            // TODO: integrate arc fill rendering when rasterizer support lands.
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Line

pub struct Line;

impl Line {
    pub fn new() -> LineBuilder {
        LineBuilder::default()
    }
}

pub struct LineBuilder {
    vertices: Option<Vertices>,
    stroke: Option<StrokePlan>,
    caps: LineCaps,
    dash: Option<DashPattern>,
    clip: Option<Area>,
}

impl Default for LineBuilder {
    fn default() -> Self {
        Self {
            vertices: None,
            stroke: None,
            caps: LineCaps::butt(),
            dash: None,
            clip: None,
        }
    }
}

impl LineBuilder {
    pub fn vertices(mut self, vertices: Vertices) -> Self {
        self.vertices = Some(vertices);
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn caps(mut self, caps: LineCaps) -> Self {
        self.caps = caps;
        self
    }

    pub fn dash(mut self, pattern: DashPattern) -> Self {
        self.dash = (!pattern.is_empty()).then_some(pattern);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> LinePrimitive {
        let vertices = self
            .vertices
            .expect("Line::vertices(Vertices::new) must be specified before finish()");

        LinePrimitive {
            vertices,
            stroke: self.stroke,
            caps: self.caps,
            dash: self.dash,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LinePrimitive {
    vertices: Vertices,
    stroke: Option<StrokePlan>,
    caps: LineCaps,
    dash: Option<DashPattern>,
    clip: Option<Area>,
}

impl LinePrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        let points = self.vertices.as_slice();
        if points.len() < 2 {
            return;
        }

        let stroke = match &self.stroke {
            Some(stroke)
                if stroke.opacity > 0 && (stroke.color.is_some() || stroke.gradient.is_some()) =>
            {
                stroke
            }
            _ => return,
        };

        let mut dsc = LineDsc::new(points[0], points[1]);
        dsc.width = stroke.width.max(1);
        dsc.opa = stroke.opacity;
        dsc.color = stroke
            .color
            .unwrap_or_else(|| fallback_gradient_color(stroke.gradient.as_ref()));

        dsc.round_start = matches!(self.caps.start, LineCap::Round);
        dsc.round_end = matches!(self.caps.end, LineCap::Round);

        if let Some(pattern) = &self.dash {
            let mut iter = pattern.iter();
            if let Some(on) = iter.next() {
                dsc.dash_width = on.raw();
            }
            if let Some(off) = iter.next() {
                dsc.dash_gap = off.raw();
            }
        }

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_line(&mut clipped, &dsc);
            }
        } else {
            draw_line(rast, &dsc);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Triangle

pub struct Triangle;

impl Triangle {
    pub fn new() -> TriangleBuilder {
        TriangleBuilder::default()
    }
}

#[derive(Default)]
pub struct TriangleBuilder {
    vertices: Option<Vertices>,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
}

impl TriangleBuilder {
    pub fn vertices(mut self, vertices: Vertices) -> Self {
        self.vertices = Some(vertices);
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn fill(mut self, fill: FillPlan) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> TrianglePrimitive {
        let vertices = self
            .vertices
            .expect("Triangle::vertices(Vertices::new3) must be specified before finish()");

        TrianglePrimitive {
            vertices,
            stroke: self.stroke,
            fill: self.fill,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TrianglePrimitive {
    vertices: Vertices,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
}

impl TrianglePrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        let points = self.vertices.as_slice();
        if points.len() < 3 {
            return;
        }

        let mut dsc = TriangleDsc::new(points[0], points[1], points[2]);
        match self.fill.as_ref() {
            Some(FillPlan::Solid { color, opacity }) => {
                dsc.color = *color;
                dsc.opa = *opacity;
                dsc.grad = types::Gradient::none();
            }
            Some(FillPlan::Gradient { gradient }) => {
                dsc.color = gradient
                    .stops()
                    .first()
                    .map(|stop| stop.color)
                    .unwrap_or(Rgba8888::WHITE);
                dsc.opa = OPA_COVER;
                dsc.grad = convert_gradient(gradient);
            }
            None | Some(FillPlan::None) => {
                dsc.opa = 0;
            }
        }

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_triangle(&mut clipped, &dsc);
                render_triangle_stroke(&mut clipped, points, self.stroke.as_ref());
            }
        } else {
            draw_triangle(rast, &dsc);
            render_triangle_stroke(rast, points, self.stroke.as_ref());
        }
    }
}

fn render_triangle_stroke<R: Rasterizer>(
    rast: &mut R,
    points: &[Point],
    stroke: Option<&StrokePlan>,
) {
    let Some(stroke) = stroke else {
        return;
    };
    if stroke.width <= 0 || stroke.opacity == 0 {
        return;
    }

    let mut draw_edge = |start: Point, end: Point| {
        let mut dsc = LineDsc::new(start, end);
        dsc.width = stroke.width.max(1);
        dsc.opa = stroke.opacity;
        dsc.color = stroke
            .color
            .unwrap_or_else(|| fallback_gradient_color(stroke.gradient.as_ref()));
        dsc.round_start = true;
        dsc.round_end = true;
        draw_line(rast, &dsc);
    };

    draw_edge(points[0], points[1]);
    draw_edge(points[1], points[2]);
    draw_edge(points[2], points[0]);
}

// -------------------------------------------------------------------------------------------------
// Label

pub struct Label;

impl Label {
    pub fn new() -> LabelBuilder {
        LabelBuilder::default()
    }
}

pub struct LabelBuilder {
    origin: Option<Point>,
    content: Option<LabelContent>,
    alignment: LabelAlignment,
    decor: LabelDecor,
    spacing: LabelSpacing,
    opacity: LabelOpacity,
    color: Option<Rgba8888>,
    clip: Option<Area>,
}

impl Default for LabelBuilder {
    fn default() -> Self {
        Self {
            origin: None,
            content: None,
            alignment: LabelAlignment::Start,
            decor: LabelDecor::None,
            spacing: LabelSpacing::new(0, 0),
            opacity: LabelOpacity::new(OPA_COVER),
            color: None,
            clip: None,
        }
    }
}

impl LabelBuilder {
    pub fn origin(mut self, origin: Point) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn content<'a>(mut self, content: LabelContentPlan<'a>) -> Self {
        self.content = Some(content.into_owned());
        self
    }

    pub fn alignment(mut self, alignment: LabelAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn decor(mut self, decor: LabelDecor) -> Self {
        self.decor = decor;
        self
    }

    pub fn spacing(mut self, spacing: LabelSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn opacity(mut self, opacity: LabelOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn color(mut self, color: Rgba8888) -> Self {
        self.color = Some(color);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> LabelPrimitive {
        let origin = self
            .origin
            .expect("Label::origin(Point) must be specified before finish()");
        let content = self
            .content
            .expect("Label::content(...) must be specified before finish()");

        LabelPrimitive {
            origin,
            content,
            alignment: self.alignment,
            decor: self.decor,
            spacing: self.spacing,
            opacity: self.opacity,
            color: self.color.unwrap_or(Rgba8888::WHITE),
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LabelPrimitive {
    origin: Point,
    content: LabelContent,
    alignment: LabelAlignment,
    decor: LabelDecor,
    spacing: LabelSpacing,
    opacity: LabelOpacity,
    color: Rgba8888,
    clip: Option<Area>,
}

impl LabelPrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        let (font_handle, text) = match &self.content {
            LabelContent::Text { font, text } => (font, text),
        };

        let font_id = resolve_font(font_handle);
        let letter_space = self.spacing.letter();
        let width = measure_text_with_font(text, letter_space, font_id).max(1);
        let height = line_height_for_font(font_id).max(1);

        let (x1, x2) = match self.alignment {
            LabelAlignment::Start => (self.origin.x, self.origin.x + width - 1),
            LabelAlignment::Center => {
                let half = width / 2;
                let start = self.origin.x - half;
                (start, start + width - 1)
            }
            LabelAlignment::End => (self.origin.x - width + 1, self.origin.x),
        };
        let y1 = self.origin.y;
        let y2 = self.origin.y + height - 1;
        let area = Area::new(x1, y1, x2, y2);

        let mut dsc = LabelDsc::new(text.clone());
        dsc.color = self.color;
        dsc.opa = self.opacity.value();
        dsc.decor = convert_decor(self.decor);
        dsc.letter_space = letter_space;
        dsc.font = font_id;

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_label(&mut clipped, &dsc, &area);
            }
        } else {
            draw_label(rast, &dsc, &area);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Blur

pub struct Blur;

impl Blur {
    pub fn new() -> BlurBuilder {
        BlurBuilder::default()
    }
}

#[derive(Default)]
pub struct BlurBuilder {
    area: Option<Area>,
    radius: i32,
    corner_radius: i32,
    clip: Option<Area>,
}

impl BlurBuilder {
    pub fn area(mut self, area: Area) -> Self {
        self.area = Some(area);
        self
    }

    pub fn radius(mut self, radius: i32) -> Self {
        self.radius = radius.max(0);
        self
    }

    pub fn corner_radius(mut self, radius: i32) -> Self {
        self.corner_radius = radius.max(0);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> BlurPrimitive {
        let area = self
            .area
            .expect("Blur::area(Area) must be specified before finish()");

        BlurPrimitive {
            area,
            radius: self.radius,
            corner_radius: self.corner_radius,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlurPrimitive {
    area: Area,
    radius: i32,
    corner_radius: i32,
    clip: Option<Area>,
}

impl BlurPrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        if self.radius <= 0 {
            return;
        }

        let mut dsc = BlurDsc::new(self.radius);
        dsc.corner_radius = self.corner_radius;

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_blur(&mut clipped, &dsc, &self.area);
            }
        } else {
            draw_blur(rast, &dsc, &self.area);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Vector

pub struct Vector;

impl Vector {
    pub fn new() -> VectorBuilder {
        VectorBuilder::default()
    }
}

pub struct VectorBuilder {
    path: Option<PathPlan<'static>>,
    winding: WindingPlan,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
}

impl Default for VectorBuilder {
    fn default() -> Self {
        Self {
            path: None,
            winding: WindingPlan::NonZero,
            stroke: None,
            fill: None,
            clip: None,
        }
    }
}

impl VectorBuilder {
    pub fn path<'a>(mut self, path: PathPlan<'a>) -> Self {
        self.path = Some(path.into_owned());
        self
    }

    pub fn winding(mut self, winding: WindingPlan) -> Self {
        self.winding = winding;
        self
    }

    pub fn stroke(mut self, stroke: StrokePlan) -> Self {
        self.stroke = Some(stroke);
        self
    }

    pub fn fill(mut self, fill: FillPlan) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn clip(mut self, clip: Area) -> Self {
        self.clip = Some(clip);
        self
    }

    pub fn finish(self) -> VectorPrimitive {
        let plan = self
            .path
            .expect("Vector::path(PathPlan::from_svg) must be specified before finish()");
        let paths = parse_svg_paths(&plan)
            .expect("Vector path contains unsupported commands for this renderer");

        VectorPrimitive {
            paths,
            winding: self.winding,
            stroke: self.stroke,
            fill: self.fill,
            clip: self.clip,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VectorPrimitive {
    paths: Vec<VectorPath>,
    winding: WindingPlan,
    stroke: Option<StrokePlan>,
    fill: Option<FillPlan>,
    clip: Option<Area>,
}

impl VectorPrimitive {
    pub fn draw<R: Rasterizer>(&self, rast: &mut R) {
        if self.paths.is_empty() {
            return;
        }

        let mut dsc = VectorDsc::new();
        for path in &self.paths {
            dsc.add_path(path.clone());
        }

        dsc.fill = convert_vector_fill(self.fill.as_ref(), self.winding);
        dsc.stroke = convert_vector_stroke(self.stroke.as_ref());

        if let Some(clip) = self.clip {
            if let Some(clamped) = clip_to_raster(&clip, &*rast) {
                let mut clipped = FluentClipper::new(rast, clamped);
                draw_vector(&mut clipped, &dsc);
            }
        } else {
            draw_vector(rast, &dsc);
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Helper conversions

fn apply_radius(dsc: &mut RectDsc, radius: Radius) {
    dsc.radius = match radius {
        Radius::Uniform(value) => value,
        Radius::Corners { top_left, .. } => top_left,
        Radius::Ring { outer, .. } => outer,
    };
}

fn apply_fill(dsc: &mut RectDsc, fill: Option<&FillPlan>) {
    match fill {
        Some(FillPlan::Solid { color, opacity }) => {
            dsc.bg_color = *color;
            dsc.bg_opa = *opacity;
            dsc.bg_grad = types::Gradient::none();
        }
        Some(FillPlan::Gradient { gradient }) => {
            dsc.bg_opa = OPA_COVER;
            dsc.bg_color = gradient
                .stops()
                .first()
                .map(|stop| stop.color)
                .unwrap_or(Rgba8888::WHITE);
            dsc.bg_grad = convert_gradient(gradient);
        }
        None | Some(FillPlan::None) => {
            dsc.bg_opa = 0;
        }
    }
}

fn apply_stroke(dsc: &mut RectDsc, stroke: Option<&StrokePlan>) {
    if let Some(stroke) = stroke {
        if stroke.width > 0 && stroke.opacity > 0 {
            dsc.border_width = stroke.width;
            dsc.border_opa = stroke.opacity;
            dsc.border_color = stroke
                .color
                .unwrap_or_else(|| fallback_gradient_color(stroke.gradient.as_ref()));
            dsc.border_side = stroke.sides;
        }
    }
}

fn apply_outline(dsc: &mut RectDsc, outline: Option<&OutlinePlan>) {
    if let Some(outline) = outline {
        let stroke = &outline.stroke;
        if stroke.width > 0 && stroke.opacity > 0 {
            dsc.outline_width = stroke.width;
            dsc.outline_opa = stroke.opacity;
            dsc.outline_color = stroke
                .color
                .unwrap_or_else(|| fallback_gradient_color(stroke.gradient.as_ref()));
            dsc.outline_pad = outline.pad;
        }
    }
}

fn apply_shadow(dsc: &mut RectDsc, shadow: Option<&ShadowPlan>) {
    if let Some(shadow) = shadow {
        if shadow.blur_radius > 0 && shadow.opacity > 0 {
            dsc.shadow_width = shadow.blur_radius;
            dsc.shadow_opa = shadow.opacity;
            dsc.shadow_color = shadow.color;
            dsc.shadow_offset_x = shadow.offset.x;
            dsc.shadow_offset_y = shadow.offset.y;
            dsc.shadow_spread = shadow.spread;
        }
    }
}

fn fallback_gradient_color(plan: Option<&GradientPlan>) -> Rgba8888 {
    plan.and_then(|grad| grad.stops().first().copied())
        .map(|stop| stop.color)
        .unwrap_or(Rgba8888::WHITE)
}

fn convert_gradient(plan: &GradientPlan) -> types::Gradient {
    let dir = match plan.kind {
        GradientKind::Linear => match plan.axis {
            Axis::Horizontal => types::GradDir::Hor,
            Axis::Vertical => types::GradDir::Ver,
        },
        GradientKind::Radial => types::GradDir::Radial,
        GradientKind::Conic => types::GradDir::Conical,
    };

    let mut gradient = types::Gradient::none();
    gradient.dir = dir;
    let mut count = 0;
    for stop in plan.stops().iter().take(types::MAX_GRADIENT_STOPS) {
        let frac = (stop.position.clamp(0.0, 1.0) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
        gradient.stops[count] = types::GradStop {
            color: stop.color,
            opa: stop.opacity,
            frac,
        };
        count += 1;
    }
    if count == 0 {
        gradient.stops[0] = types::GradStop::default();
        gradient.stops_count = 1;
    } else {
        gradient.stops_count = count;
    }
    gradient
}

fn resolve_font(handle: &FontHandle<'static>) -> FontId {
    const TABLE: &[(u8, FontId)] = &[
        (8, FontId::Montserrat8),
        (10, FontId::Montserrat10),
        (12, FontId::Montserrat12),
        (14, FontId::Montserrat14),
        (16, FontId::Montserrat16),
        (18, FontId::Montserrat18),
        (20, FontId::Montserrat20),
        (22, FontId::Montserrat22),
        (24, FontId::Montserrat24),
        (26, FontId::Montserrat26),
        (28, FontId::Montserrat28),
        (30, FontId::Montserrat30),
        (32, FontId::Montserrat32),
        (34, FontId::Montserrat34),
        (36, FontId::Montserrat36),
        (38, FontId::Montserrat38),
        (40, FontId::Montserrat40),
        (42, FontId::Montserrat42),
        (44, FontId::Montserrat44),
        (46, FontId::Montserrat46),
        (48, FontId::Montserrat48),
    ];

    let size = handle.size_px;
    let mut best = TABLE[0];
    let mut best_delta = (size as i32 - TABLE[0].0 as i32).abs();
    for &(candidate, id) in TABLE.iter() {
        let delta = (size as i32 - candidate as i32).abs();
        if delta < best_delta {
            best = (candidate, id);
            best_delta = delta;
        }
    }
    best.1
}

fn convert_decor(decor: LabelDecor) -> TextDecor {
    match decor {
        LabelDecor::None => TextDecor::None,
        LabelDecor::Underline => TextDecor::Underline,
        LabelDecor::Strikethrough => TextDecor::Strikethrough,
    }
}

fn parse_svg_paths(plan: &PathPlan<'static>) -> Result<Vec<VectorPath>, ()> {
    let tokens = tokenize_svg(&plan.svg)?;
    build_vector_paths(&tokens)
}

#[derive(Clone, Debug)]
enum SvgToken {
    Command(char),
    Number(f32),
}

fn tokenize_svg(data: &str) -> Result<Vec<SvgToken>, ()> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_number = false;

    for ch in data.chars() {
        if ch.is_ascii_alphabetic() {
            if in_number {
                push_number(&mut current, &mut tokens)?;
                current.clear();
                in_number = false;
            }
            let cmd = ch.to_ascii_uppercase();
            match cmd {
                'M' | 'L' | 'C' | 'Z' => tokens.push(SvgToken::Command(cmd)),
                _ => return Err(()),
            }
        } else if ch.is_ascii_digit() || ch == '-' || ch == '.' {
            current.push(ch);
            in_number = true;
        } else if ch == ',' || ch.is_whitespace() {
            if in_number {
                push_number(&mut current, &mut tokens)?;
                current.clear();
                in_number = false;
            }
        } else {
            return Err(());
        }
    }

    if in_number {
        push_number(&mut current, &mut tokens)?;
    }

    Ok(tokens)
}

fn push_number(buffer: &mut String, tokens: &mut Vec<SvgToken>) -> Result<(), ()> {
    if buffer.is_empty() {
        return Ok(());
    }
    let value: f32 = buffer.parse().map_err(|_| ())?;
    tokens.push(SvgToken::Number(value));
    Ok(())
}

fn build_vector_paths(tokens: &[SvgToken]) -> Result<Vec<VectorPath>, ()> {
    let mut paths = Vec::new();
    let mut current = VectorPath::new();
    let mut idx = 0;
    let mut current_cmd: Option<char> = None;

    while idx < tokens.len() {
        match tokens[idx] {
            SvgToken::Command(cmd) => match cmd {
                'Z' => {
                    current.close();
                    current_cmd = None;
                    idx += 1;
                }
                _ => {
                    current_cmd = Some(cmd);
                    idx += 1;
                }
            },
            SvgToken::Number(_) => {
                let cmd = current_cmd.ok_or(())?;
                match cmd {
                    'M' => {
                        let (x, y, consumed) = take_coord_pair(tokens, idx)?;
                        if !current.is_empty() {
                            paths.push(current);
                            current = VectorPath::new();
                        }
                        current.move_to(FPoint::new(x, y));
                        idx += consumed;
                        current_cmd = Some('L');
                    }
                    'L' => {
                        let (x, y, consumed) = take_coord_pair(tokens, idx)?;
                        current.line_to(FPoint::new(x, y));
                        idx += consumed;
                    }
                    'C' => {
                        let (c1x, c1y, consumed1) = take_coord_pair(tokens, idx)?;
                        let (c2x, c2y, consumed2) = take_coord_pair(tokens, idx + consumed1)?;
                        let (px, py, consumed3) =
                            take_coord_pair(tokens, idx + consumed1 + consumed2)?;
                        current.cubic_to(
                            FPoint::new(c1x, c1y),
                            FPoint::new(c2x, c2y),
                            FPoint::new(px, py),
                        );
                        idx += consumed1 + consumed2 + consumed3;
                    }
                    _ => return Err(()),
                }
            }
        }
    }

    if !current.is_empty() {
        paths.push(current);
    }

    Ok(paths)
}

fn take_coord_pair(tokens: &[SvgToken], start: usize) -> Result<(f32, f32, usize), ()> {
    if start + 1 >= tokens.len() {
        return Err(());
    }
    let x = match tokens[start] {
        SvgToken::Number(value) => value,
        _ => return Err(()),
    };
    let y = match tokens[start + 1] {
        SvgToken::Number(value) => value,
        _ => return Err(()),
    };
    Ok((x, y, 2))
}

fn convert_vector_fill(fill: Option<&FillPlan>, winding: WindingPlan) -> Option<VectorFill> {
    match fill {
        None | Some(FillPlan::None) => None,
        Some(FillPlan::Solid { color, opacity }) => Some(VectorFill::solid(*color, *opacity)),
        Some(FillPlan::Gradient { gradient }) => {
            if gradient.kind != GradientKind::Linear {
                return None;
            }
            let stops: Vec<VectorColorStop> = gradient
                .stops()
                .iter()
                .map(|stop| VectorColorStop {
                    color: stop.color,
                    opa: stop.opacity,
                    frac: (stop.position.clamp(0.0, 1.0) * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8,
                })
                .collect();
            if stops.len() < 2 {
                return stops
                    .first()
                    .map(|stop| VectorFill::solid(stop.color, stop.opa));
            }
            let start = gradient
                .start_point
                .map(|p| FPoint::new(p.x as f32, p.y as f32))
                .unwrap_or_else(|| match gradient.axis {
                    Axis::Horizontal => FPoint::new(0.0, 0.0),
                    Axis::Vertical => FPoint::new(0.0, 0.0),
                });
            let end = gradient
                .end_point
                .map(|p| FPoint::new(p.x as f32, p.y as f32))
                .unwrap_or_else(|| match gradient.axis {
                    Axis::Horizontal => FPoint::new(1.0, 0.0),
                    Axis::Vertical => FPoint::new(0.0, 1.0),
                });
            let gradient = VectorLinearGradient { start, end, stops };
            let rule = match winding {
                WindingPlan::NonZero => VectorFillRule::NonZero,
                WindingPlan::EvenOdd => VectorFillRule::EvenOdd,
            };
            Some(VectorFill::linear_gradient(gradient, rule))
        }
    }
}

fn convert_vector_stroke(stroke: Option<&StrokePlan>) -> Option<VectorStroke> {
    let stroke = stroke?;
    if stroke.width <= 0 || stroke.opacity == 0 {
        return None;
    }

    let mut vector_stroke = if let Some(color) = stroke.color {
        VectorStroke::new(stroke.width as f32, color, stroke.opacity)
    } else if let Some(gradient) = &stroke.gradient {
        if gradient.kind != GradientKind::Linear {
            return None;
        }
        let stops: Vec<VectorColorStop> = gradient
            .stops()
            .iter()
            .map(|stop| VectorColorStop {
                color: stop.color,
                opa: stop.opacity,
                frac: (stop.position.clamp(0.0, 1.0) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8,
            })
            .collect();
        let start = gradient
            .start_point
            .map(|p| FPoint::new(p.x as f32, p.y as f32))
            .unwrap_or_else(|| match gradient.axis {
                Axis::Horizontal => FPoint::new(0.0, 0.0),
                Axis::Vertical => FPoint::new(0.0, 0.0),
            });
        let end = gradient
            .end_point
            .map(|p| FPoint::new(p.x as f32, p.y as f32))
            .unwrap_or_else(|| match gradient.axis {
                Axis::Horizontal => FPoint::new(1.0, 0.0),
                Axis::Vertical => FPoint::new(0.0, 1.0),
            });
        let gradient = VectorLinearGradient { start, end, stops };
        VectorStroke::with_gradient(stroke.width as f32, gradient, stroke.opacity)
    } else {
        return None;
    };

    if let Some(caps) = stroke.caps {
        let round = matches!(caps.start, LineCap::Round) || matches!(caps.end, LineCap::Round);
        vector_stroke.cap = if round {
            VectorStrokeCap::Round
        } else {
            VectorStrokeCap::Butt
        };
    }

    if let Some(join) = stroke.join {
        vector_stroke.join = match join {
            StrokeJoinStyle::Miter => VectorStrokeJoin::Miter,
            StrokeJoinStyle::Round => VectorStrokeJoin::Round,
        };
    }

    if let Some(pattern) = &stroke.dash {
        vector_stroke.dash_pattern = pattern.clone();
    }

    if stroke.caps.is_none() {
        vector_stroke.cap = VectorStrokeCap::Butt;
    }
    if stroke.join.is_none() {
        vector_stroke.join = VectorStrokeJoin::Miter;
    }
    Some(vector_stroke)
}

// -------------------------------------------------------------------------------------------------
