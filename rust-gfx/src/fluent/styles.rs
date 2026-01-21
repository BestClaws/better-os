//! Shared style builders for the fluent primitives.

extern crate alloc;

use alloc::vec::Vec;

use crate::color::Rgba8888;
use crate::types::{BorderSide, Opa, OPA_COVER, Point};

use micromath::F32Ext;

use super::types::{Angle, LineCaps};

/// Maximum number of gradient stops supported by the fluent builders.
pub const MAX_GRADIENT_STOPS: usize = 8;

/// Direction used by linear gradients.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// Gradient families offered by the fluent builders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
    Linear,

    Radial,

    Conic,
}

/// Gradient color stop with fractional position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub position: f32,
    pub color: Rgba8888,
    pub opacity: Opa,
}

impl GradientStop {
    pub fn new(position: f32, color: Rgba8888) -> Self {
        Self {
            position,
            color,
            opacity: OPA_COVER,
        }
    }

    pub fn with_opacity(mut self, opacity: Opa) -> Self {
        self.opacity = opacity;
        self
    }
}

/// Completed gradient definition ready for conversion.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientPlan {
    pub kind: GradientKind,
    pub axis: Axis,
    pub center: Option<crate::types::Point>,
    pub radius: Option<i32>,
    pub start_angle: Option<Angle>,
    pub start_point: Option<Point>,
    pub end_point: Option<Point>,
    pub stops: [GradientStop; MAX_GRADIENT_STOPS],
    pub stop_count: usize,
}

impl GradientPlan {
    pub fn stops(&self) -> &[GradientStop] {
        &self.stops[..self.stop_count]
    }
}

/// Builder for creating gradient plans.
#[derive(Clone, Debug)]
pub struct GradientBuilder {
    plan: GradientPlan,
}

impl GradientBuilder {
    pub fn linear() -> Self {
        Self {
            plan: GradientPlan {
                kind: GradientKind::Linear,
                axis: Axis::Horizontal,
                center: None,
                radius: None,
                start_angle: None,
                start_point: None,
                end_point: None,
                stops: [GradientStop::new(0.0, Rgba8888::BLACK); MAX_GRADIENT_STOPS],
                stop_count: 0,
            },
        }
    }

    pub fn radial() -> Self {
        Self::linear().kind(GradientKind::Radial)
    }

    pub fn conic() -> Self {
        Self::linear().kind(GradientKind::Conic)
    }

    pub fn kind(mut self, kind: GradientKind) -> Self {
        self.plan.kind = kind;
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.plan.axis = axis;
        self
    }

    pub fn center(mut self, center: crate::types::Point) -> Self {
        self.plan.center = Some(center);
        self
    }

    pub fn radius(mut self, radius: i32) -> Self {
        self.plan.radius = Some(radius.max(0));
        self
    }

    pub fn start_angle(mut self, angle: Angle) -> Self {
        self.plan.start_angle = Some(angle);
        self
    }

    pub fn start_point(mut self, point: Point) -> Self {
        self.plan.start_point = Some(point);
        self
    }

    pub fn end_point(mut self, point: Point) -> Self {
        self.plan.end_point = Some(point);
        self
    }

    pub fn push_stop(mut self, stop: GradientStop) -> Self {
        if self.plan.stop_count < MAX_GRADIENT_STOPS {
            self.plan.stops[self.plan.stop_count] = stop;
            self.plan.stop_count += 1;
        }
        self
    }

    pub fn stops<const N: usize>(mut self, stops: [GradientStop; N]) -> Self {
        for stop in stops.into_iter().take(MAX_GRADIENT_STOPS) {
            self = self.push_stop(stop);
        }
        self
    }

    pub fn finish(mut self) -> GradientPlan {
        if self.plan.stop_count == 0 {
            self.plan.stops[0] = GradientStop::new(0.0, Rgba8888::WHITE);
            self.plan.stop_count = 1;
        }
        self.plan
    }
}

/// Stroke definition shared across primitives.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokePlan {
    pub width: i32,
    pub color: Option<Rgba8888>,
    pub opacity: Opa,
    pub gradient: Option<GradientPlan>,
    pub sides: BorderSide,
    pub caps: Option<LineCaps>,
    pub join: Option<StrokeJoinStyle>,
    pub dash: Option<Vec<f32>>,
}

impl StrokePlan {
    pub fn is_none(&self) -> bool {
        self.width <= 0 || (self.color.is_none() && self.gradient.is_none()) || self.opacity == 0
    }
}

/// Join style applied when stroking vector paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrokeJoinStyle {
    Miter,
    Round,
}

#[derive(Clone, Debug)]
pub struct StrokeBuilder {
    width: i32,
    color: Option<Rgba8888>,
    opacity: Opa,
    gradient: Option<GradientPlan>,
    sides: BorderSide,
    caps: Option<LineCaps>,
    join: Option<StrokeJoinStyle>,
    dash: Option<Vec<f32>>,
}

impl StrokeBuilder {
    pub fn new() -> Self {
        Self {
            width: 0,
            color: None,
            opacity: OPA_COVER,
            gradient: None,
            sides: BorderSide::FULL,
            caps: None,
            join: None,
            dash: None,
        }
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width.max(0);
        self
    }

    pub fn color(mut self, color: Rgba8888) -> Self {
        self.color = Some(color);
        self.gradient = None;
        self
    }

    pub fn opacity(mut self, opacity: Opa) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn gradient(mut self, gradient: GradientPlan) -> Self {
        self.gradient = Some(gradient);
        self.color = None;
        self
    }

    pub fn sides(mut self, sides: BorderSide) -> Self {
        self.sides = sides;
        self
    }

    pub fn caps(mut self, caps: LineCaps) -> Self {
        self.caps = Some(caps);
        self
    }

    pub fn join(mut self, join: StrokeJoinStyle) -> Self {
        self.join = Some(join);
        self
    }

    pub fn dash_pattern(mut self, pattern: &[f32]) -> Self {
        if pattern.is_empty() {
            self.dash = None;
        } else {
            self.dash = Some(pattern.to_vec());
        }
        self
    }

    pub fn finish(self) -> StrokePlan {
        StrokePlan {
            width: self.width,
            color: self.color,
            opacity: self.opacity,
            gradient: self.gradient,
            sides: self.sides,
            caps: self.caps,
            join: self.join,
            dash: self.dash,
        }
    }
}

impl StrokePlan {
    pub fn solid(width: i32, color: Rgba8888) -> Self {
        Self {
            width: width.max(0),
            color: Some(color),
            opacity: OPA_COVER,
            gradient: None,
            sides: BorderSide::FULL,
            caps: None,
            join: None,
            dash: None,
        }
    }
}

/// Fill definition shared across primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum FillPlan {
    None,
    Solid { color: Rgba8888, opacity: Opa },
    Gradient { gradient: GradientPlan },
}

impl FillPlan {
    pub fn solid(color: Rgba8888) -> Self {
        Self::Solid {
            color,
            opacity: OPA_COVER,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FillBuilder {
    plan: FillPlan,
    opacity: Opa,
}

impl FillBuilder {
    pub fn solid(color: Rgba8888) -> Self {
        Self {
            plan: FillPlan::Solid {
                color,
                opacity: OPA_COVER,
            },
            opacity: OPA_COVER,
        }
    }

    pub fn gradient() -> GradientFillBuilder {
        GradientFillBuilder {
            base: GradientBuilder::linear(),
        }
    }

    pub fn opacity(mut self, opacity: Opa) -> Self {
        self.opacity = opacity;
        if let FillPlan::Solid { color, .. } = self.plan {
            self.plan = FillPlan::Solid { color, opacity };
        }
        self
    }

    pub fn finish(self) -> FillPlan {
        match self.plan {
            FillPlan::Solid { color, .. } => FillPlan::Solid {
                color,
                opacity: self.opacity,
            },
            _ => self.plan,
        }
    }
}

pub struct GradientFillBuilder {
    base: GradientBuilder,
}

impl GradientFillBuilder {
    pub fn axis(mut self, axis: Axis) -> Self {
        self.base = self.base.axis(axis);
        self
    }

    pub fn kind(mut self, kind: GradientKind) -> Self {
        self.base = self.base.kind(kind);
        self
    }

    pub fn center(mut self, center: crate::types::Point) -> Self {
        self.base = self.base.center(center);
        self
    }

    pub fn radius(mut self, radius: i32) -> Self {
        self.base = self.base.radius(radius);
        self
    }

    pub fn start_angle(mut self, angle: Angle) -> Self {
        self.base = self.base.start_angle(angle);
        self
    }

    pub fn stops<const N: usize>(mut self, stops: [GradientStop; N]) -> Self {
        self.base = self.base.stops(stops);
        self
    }

    pub fn finish(self) -> FillPlan {
        FillPlan::Gradient {
            gradient: self.base.finish(),
        }
    }
}

/// Outline definition wrapping a stroke plan.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlinePlan {
    pub stroke: StrokePlan,
    pub pad: i32,
}

#[derive(Clone, Debug)]
pub struct OutlineBuilder {
    stroke: StrokePlan,
    pad: i32,
}

impl OutlineBuilder {
    pub fn with_stroke(stroke: StrokePlan) -> Self {
        Self { stroke, pad: 0 }
    }

    pub fn pad(mut self, pad: i32) -> Self {
        self.pad = pad.max(0);
        self
    }

    pub fn finish(self) -> OutlinePlan {
        OutlinePlan {
            stroke: self.stroke,
            pad: self.pad,
        }
    }
}

/// Drop shadow definition used by rectangle and circle primitives.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowPlan {
    pub offset: crate::types::Point,
    pub blur_radius: i32,
    pub color: Rgba8888,
    pub opacity: Opa,
    pub spread: i32,
}

#[derive(Clone, Debug)]
pub struct ShadowBuilder {
    offset: crate::types::Point,
    blur_radius: i32,
    color: Rgba8888,
    opacity: Opa,
    spread: i32,
}

impl ShadowBuilder {
    pub fn new() -> Self {
        Self {
            offset: crate::types::Point::new(0, 0),
            blur_radius: 0,
            color: Rgba8888::BLACK,
            opacity: OPA_COVER,
            spread: 0,
        }
    }

    pub fn offset(mut self, offset: crate::types::Point) -> Self {
        self.offset = offset;
        self
    }

    pub fn blur(mut self, radius: i32) -> Self {
        self.blur_radius = radius.max(0);
        self
    }

    pub fn color(mut self, color: Rgba8888) -> Self {
        self.color = color;
        self
    }

    pub fn opacity(mut self, opacity: Opa) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn spread(mut self, spread: i32) -> Self {
        self.spread = spread.max(0);
        self
    }

    pub fn finish(self) -> ShadowPlan {
        ShadowPlan {
            offset: self.offset,
            blur_radius: self.blur_radius,
            color: self.color,
            opacity: self.opacity,
            spread: self.spread,
        }
    }
}

/// Internal helper converting stroke gradients to a simple lookup table per span.
pub fn build_gradient_row(gradient: &GradientPlan, span: usize) -> Vec<(Rgba8888, Opa)> {
    let mut table = Vec::with_capacity(span);
    if span == 0 {
        return table;
    }

    for i in 0..span {
        let t = i as f32 / (span.saturating_sub(1) as f32).max(1.0);
        let (color, opa) = sample_gradient(gradient, t);
        table.push((color, opa));
    }
    table
}

fn sample_gradient(gradient: &GradientPlan, t: f32) -> (Rgba8888, Opa) {
    let stops = gradient.stops();
    if stops.is_empty() {
        return (Rgba8888::BLACK, 0);
    }

    if stops.len() == 1 {
        let stop = stops[0];
        return (stop.color, stop.opacity);
    }

    let mut prev = stops[0];
    for stop in &stops[1..] {
        if t <= stop.position {
            let span = (stop.position - prev.position).max(1e-6);
            let factor = ((t - prev.position) / span).clamp(0.0, 1.0);
            return lerp_stop(prev, *stop, factor);
        }
        prev = *stop;
    }
    (prev.color, prev.opacity)
}

fn lerp_stop(a: GradientStop, b: GradientStop, t: f32) -> (Rgba8888, Opa) {
    let lerp = |x0: u8, x1: u8| -> u8 {
        (x0 as f32 + (x1 as f32 - x0 as f32) * t).round() as u8
    };
    let color = Rgba8888::rgba(
        lerp(a.color.r(), b.color.r()),
        lerp(a.color.g(), b.color.g()),
        lerp(a.color.b(), b.color.b()),
        lerp(a.color.a(), b.color.a()),
    );
    let opa = (a.opacity as f32 + (b.opacity as f32 - a.opacity as f32) * t).round() as Opa;
    (color, opa)
}
