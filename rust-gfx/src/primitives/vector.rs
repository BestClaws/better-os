use alloc::vec;
use alloc::vec::Vec;
use core::cmp::{max, min};
use core::mem;

use micromath::F32Ext;

use crate::color::Rgba8888;
use crate::types::{Opa, OPA_COVER};
use crate::Rasterizer;

const SAMPLE_GRID: usize = 4; // 4x4 supersampling for antialiasing
const SAMPLE_COUNT: usize = SAMPLE_GRID * SAMPLE_GRID;
const SAMPLE_OFFSETS: [f32; SAMPLE_GRID] = [0.125, 0.375, 0.625, 0.875];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FPoint {
    pub x: f32,
    pub y: f32,
}

fn distance(a: FPoint, b: FPoint) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

impl FPoint {
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    fn lerp(self, other: FPoint, t: f32) -> FPoint {
        FPoint::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }
}

#[derive(Clone, Debug)]
pub enum PathCmd {
    MoveTo(FPoint),
    LineTo(FPoint),
    CubicTo(FPoint, FPoint, FPoint),
    Close,
}

#[derive(Clone, Debug)]
pub struct VectorPath {
    commands: Vec<PathCmd>,
}

impl VectorPath {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn move_to(&mut self, point: FPoint) {
        self.commands.push(PathCmd::MoveTo(point));
    }

    pub fn line_to(&mut self, point: FPoint) {
        self.commands.push(PathCmd::LineTo(point));
    }

    pub fn cubic_to(&mut self, ctrl1: FPoint, ctrl2: FPoint, point: FPoint) {
        self.commands.push(PathCmd::CubicTo(ctrl1, ctrl2, point));
    }

    pub fn close(&mut self) {
        self.commands.push(PathCmd::Close);
    }

    pub fn commands(&self) -> &[PathCmd] {
        &self.commands
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FillRule {
    EvenOdd,
    NonZero,
}

impl Default for FillRule {
    fn default() -> Self {
        FillRule::NonZero
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ColorStop {
    pub color: Rgba8888,
    pub opa: Opa,
    pub frac: u8,
}

#[derive(Clone, Debug)]
pub struct LinearGradient {
    pub start: FPoint,
    pub end: FPoint,
    pub stops: Vec<ColorStop>,
}

impl LinearGradient {
    pub fn color_at(&self, point: FPoint) -> (Rgba8888, Opa) {
        if self.stops.is_empty() {
            return (Rgba8888::BLACK, 0);
        }
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let len_sq = dx * dx + dy * dy;
        let mut t = if len_sq > 0.0 {
            ((point.x - self.start.x) * dx + (point.y - self.start.y) * dy) / len_sq
        } else {
            0.0
        };
        if t < 0.0 {
            t = 0.0;
        } else if t > 1.0 {
            t = 1.0;
        }
        let frac = (t * 255.0).round() as i32;

        let mut prev = self.stops[0];
        for stop in &self.stops[1..] {
            if frac <= stop.frac as i32 {
                let span = (stop.frac as i32 - prev.frac as i32).max(1);
                let rel = ((frac - prev.frac as i32) as f32) / (span as f32);
                let r0 = prev.color.r() as f32;
                let g0 = prev.color.g() as f32;
                let b0 = prev.color.b() as f32;
                let a0 = prev.color.a() as f32;
                let r1 = stop.color.r() as f32;
                let g1 = stop.color.g() as f32;
                let b1 = stop.color.b() as f32;
                let a1 = stop.color.a() as f32;
                let r = r0 + (r1 - r0) * rel;
                let g = g0 + (g1 - g0) * rel;
                let b = b0 + (b1 - b0) * rel;
                let a = a0 + (a1 - a0) * rel;
                let opa = prev.opa as f32 + (stop.opa as f32 - prev.opa as f32) * rel;
                return (
                    Rgba8888::rgba(r.round() as u8, g.round() as u8, b.round() as u8, a.round() as u8),
                    opa.round() as Opa,
                );
            }
            prev = *stop;
        }
        (prev.color, prev.opa)
    }
}

#[derive(Clone, Debug)]
pub enum FillKind {
    Solid { color: Rgba8888, opa: Opa },
    LinearGradient(LinearGradient),
}

#[derive(Clone, Debug)]
pub struct VectorFill {
    pub kind: FillKind,
    pub rule: FillRule,
}

impl VectorFill {
    pub fn solid(color: Rgba8888, opa: Opa) -> Self {
        Self {
            kind: FillKind::Solid { color, opa },
            rule: FillRule::NonZero,
        }
    }

    pub fn linear_gradient(gradient: LinearGradient, rule: FillRule) -> Self {
        Self {
            kind: FillKind::LinearGradient(gradient),
            rule,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeCap {
    Butt,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeJoin {
    Miter,
    Round,
}

#[derive(Clone, Debug)]
pub struct StrokeGradient {
    pub gradient: LinearGradient,
}

#[derive(Clone, Debug)]
pub struct VectorStroke {
    pub width: f32,
    pub color: Option<Rgba8888>,
    pub opa: Opa,
    pub gradient: Option<StrokeGradient>,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub dash_pattern: Vec<f32>,
    pub dash_phase: f32,
}

impl VectorStroke {
    pub fn new(width: f32, color: Rgba8888, opa: Opa) -> Self {
        Self {
            width,
            color: Some(color),
            opa,
            gradient: None,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            dash_pattern: Vec::new(),
            dash_phase: 0.0,
        }
    }

    pub fn with_gradient(width: f32, gradient: LinearGradient, opa: Opa) -> Self {
        Self {
            width,
            color: None,
            opa,
            gradient: Some(StrokeGradient { gradient }),
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            dash_pattern: Vec::new(),
            dash_phase: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct VectorDsc {
    pub paths: Vec<VectorPath>,
    pub fill: Option<VectorFill>,
    pub stroke: Option<VectorStroke>,
}

impl VectorDsc {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            fill: None,
            stroke: None,
        }
    }

    pub fn add_path(&mut self, path: VectorPath) {
        if !path.is_empty() {
            self.paths.push(path);
        }
    }
}

struct Edge {
    start: FPoint,
    end: FPoint,
}

struct StrokeSegment {
    start: FPoint,
    end: FPoint,
    length: f32,
    cumulative: f32,
}

struct StrokePath {
    segments: Vec<StrokeSegment>,
    length: f32,
    closed: bool,
    start: FPoint,
    end: FPoint,
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl Bounds {
    fn new() -> Self {
        Self {
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    fn include(&mut self, p: FPoint) {
        if p.x < self.min_x {
            self.min_x = p.x;
        }
        if p.y < self.min_y {
            self.min_y = p.y;
        }
        if p.x > self.max_x {
            self.max_x = p.x;
        }
        if p.y > self.max_y {
            self.max_y = p.y;
        }
    }

    fn expand(&mut self, amount: f32) {
        self.min_x -= amount;
        self.min_y -= amount;
        self.max_x += amount;
        self.max_y += amount;
    }

    fn is_valid(&self) -> bool {
        self.min_x <= self.max_x && self.min_y <= self.max_y
    }
}

struct Flattened {
    edges: Vec<Edge>,
    stroke_paths: Vec<StrokePath>,
    bounds: Bounds,
}

pub fn draw_vector<R: Rasterizer>(rast: &mut R, dsc: &VectorDsc) {
    if dsc.paths.is_empty() {
        return;
    }

    let flattened = flatten_paths(&dsc.paths);
    if !flattened.bounds.is_valid() {
        return;
    }

    if let Some(fill) = &dsc.fill {
        render_fill(rast, fill, &flattened);
    }

    if let Some(stroke) = &dsc.stroke {
        render_stroke(rast, stroke, &flattened);
    }
}

fn flatten_paths(paths: &[VectorPath]) -> Flattened {
    let mut edges = Vec::new();
    let mut stroke_paths = Vec::new();
    let mut bounds = Bounds::new();

    for path in paths {
        let mut current = FPoint::new(0.0, 0.0);
        let mut start = FPoint::new(0.0, 0.0);
        let mut has_current = false;
        let mut stroke_points: Vec<FPoint> = Vec::new();
        let mut cumulative = 0.0f32;
        let mut segments: Vec<StrokeSegment> = Vec::new();
        let mut closed = false;

        for cmd in path.commands() {
            match *cmd {
                PathCmd::MoveTo(p) => {
                    if has_current && !stroke_points.is_empty() {
                        build_stroke_segments(&stroke_points, false, &mut segments, &mut cumulative, &mut bounds);
                        if !segments.is_empty() {
                            let start_point = stroke_points.first().copied().unwrap_or(current);
                            let end_point = stroke_points.last().copied().unwrap_or(current);
                            stroke_paths.push(StrokePath {
                                segments: mem::take(&mut segments),
                                length: cumulative,
                                closed,
                                start: start_point,
                                end: end_point,
                            });
                        }
                        stroke_points.clear();
                        cumulative = 0.0;
                        closed = false;
                    }
                    current = p;
                    start = p;
                    has_current = true;
                    stroke_points.push(p);
                    bounds.include(p);
                }
                PathCmd::LineTo(p) => {
                    if !has_current {
                        current = p;
                        start = p;
                        has_current = true;
                        stroke_points.push(p);
                        bounds.include(p);
                        continue;
                    }
                    add_edge(&mut edges, current, p);
                    stroke_points.push(p);
                    bounds.include(p);
                    current = p;
                }
                PathCmd::CubicTo(c1, c2, p) => {
                    if !has_current {
                        current = p;
                        start = p;
                        has_current = true;
                        stroke_points.push(p);
                        bounds.include(p);
                        continue;
                    }
                    let mut last = current;
                    let mut curve_points = vec![current];
                    flatten_cubic(current, c1, c2, p, &mut curve_points, 0);
                    for flat in curve_points.into_iter().skip(1) {
                        add_edge(&mut edges, last, flat);
                        stroke_points.push(flat);
                        bounds.include(flat);
                        last = flat;
                        current = flat;
                    }
                }
                PathCmd::Close => {
                    if has_current {
                        add_edge(&mut edges, current, start);
                        stroke_points.push(start);
                        current = start;
                        closed = true;
                    }
                }
            }
        }

        if has_current && stroke_points.len() >= 2 {
            build_stroke_segments(&stroke_points, closed, &mut segments, &mut cumulative, &mut bounds);
            if !segments.is_empty() {
                let start_point = stroke_points.first().copied().unwrap_or(start);
                let end_point = stroke_points.last().copied().unwrap_or(current);
                stroke_paths.push(StrokePath {
                    segments,
                    length: cumulative,
                    closed,
                    start: start_point,
                    end: end_point,
                });
            }
        }
    }

    Flattened {
        edges,
        stroke_paths,
        bounds,
    }
}

fn build_stroke_segments(
    stroke_points: &[FPoint],
    closed: bool,
    segments: &mut Vec<StrokeSegment>,
    cumulative: &mut f32,
    bounds: &mut Bounds,
) {
    if stroke_points.len() < 2 {
        return;
    }

    for window in stroke_points.windows(2) {
        let p0 = window[0];
        let p1 = window[1];
        let length = distance(p0, p1);
        if length <= f32::EPSILON {
            continue;
        }
        segments.push(StrokeSegment {
            start: p0,
            end: p1,
            length,
            cumulative: *cumulative,
        });
        *cumulative += length;
        bounds.include(p0);
        bounds.include(p1);
    }

    if closed && !segments.is_empty() {
        let first = stroke_points[0];
        let last = stroke_points[stroke_points.len() - 1];
        if distance(first, last) > f32::EPSILON {
            let length = distance(last, first);
            segments.push(StrokeSegment {
                start: last,
                end: first,
                length,
                cumulative: *cumulative,
            });
            *cumulative += length;
        }
    }
}

fn add_edge(edges: &mut Vec<Edge>, from: FPoint, to: FPoint) {
    if (from.x - to.x).abs() <= f32::EPSILON && (from.y - to.y).abs() <= f32::EPSILON {
        return;
    }
    edges.push(Edge { start: from, end: to });
}

fn flatten_cubic(p0: FPoint, p1: FPoint, p2: FPoint, p3: FPoint, out: &mut Vec<FPoint>, depth: usize) {
    if depth > 10 || cubic_is_flat(p0, p1, p2, p3) {
        out.push(p3);
        return;
    }

    let (left, right) = cubic_split(p0, p1, p2, p3);
    flatten_cubic(left[0], left[1], left[2], left[3], out, depth + 1);
    flatten_cubic(right[0], right[1], right[2], right[3], out, depth + 1);
}

fn cubic_is_flat(p0: FPoint, p1: FPoint, p2: FPoint, p3: FPoint) -> bool {
    let ux = 3.0 * p1.x - 2.0 * p0.x - p3.x;
    let uy = 3.0 * p1.y - 2.0 * p0.y - p3.y;
    let vx = 3.0 * p2.x - 2.0 * p3.x - p0.x;
    let vy = 3.0 * p2.y - 2.0 * p3.y - p0.y;
    let ux = ux * ux + uy * uy;
    let vx = vx * vx + vy * vy;
    ux.max(vx) <= 0.25
}

fn cubic_split(p0: FPoint, p1: FPoint, p2: FPoint, p3: FPoint) -> ([FPoint; 4], [FPoint; 4]) {
    let p01 = p0.lerp(p1, 0.5);
    let p12 = p1.lerp(p2, 0.5);
    let p23 = p2.lerp(p3, 0.5);
    let p012 = p01.lerp(p12, 0.5);
    let p123 = p12.lerp(p23, 0.5);
    let mid = p012.lerp(p123, 0.5);

    (
        [p0, p01, p012, mid],
        [mid, p123, p23, p3],
    )
}

fn render_fill<R: Rasterizer>(rast: &mut R, fill: &VectorFill, flattened: &Flattened) {
    if flattened.edges.is_empty() {
        return;
    }

    let mut bounds = flattened.bounds;
    bounds.expand(1.0);

    let min_x = bounds.min_x.floor() as i32;
    let max_x = bounds.max_x.ceil() as i32;
    let min_y = bounds.min_y.floor() as i32;
    let max_y = bounds.max_y.ceil() as i32;

    let mut dirty_min_x = i32::MAX;
    let mut dirty_min_y = i32::MAX;
    let mut dirty_max_x = i32::MIN;
    let mut dirty_max_y = i32::MIN;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut coverage_sum = 0;
            let mut sample_inside = false;
            for oy in &SAMPLE_OFFSETS {
                for ox in &SAMPLE_OFFSETS {
                    let sample = FPoint::new(x as f32 + ox, y as f32 + oy);
                    if point_in_path(sample, &flattened.edges, fill.rule) {
                        coverage_sum += 1;
                        sample_inside = true;
                    }
                }
            }

            if coverage_sum == 0 || !sample_inside {
                continue;
            }

            let coverage = (coverage_sum * 255) / SAMPLE_COUNT;
            let sample_point = FPoint::new(x as f32 + 0.5, y as f32 + 0.5);
            let (color, opa) = match &fill.kind {
                FillKind::Solid { color, opa } => (*color, *opa),
                FillKind::LinearGradient(grad) => grad.color_at(sample_point),
            };

            let final_opa = ((coverage as u32) * (opa as u32)) / 255;
            let final_opa = final_opa.min(255) as Opa;
            if final_opa == 0 {
                continue;
            }

            rast.blend_pixel(x, y, color, final_opa);
            if x < dirty_min_x {
                dirty_min_x = x;
            }
            if x > dirty_max_x {
                dirty_max_x = x;
            }
            if y < dirty_min_y {
                dirty_min_y = y;
            }
            if y > dirty_max_y {
                dirty_max_y = y;
            }
        }
    }

    if dirty_min_x <= dirty_max_x && dirty_min_y <= dirty_max_y {
        rast.mark_dirty(dirty_min_x, dirty_min_y, dirty_max_x + 1, dirty_max_y + 1);
    }
}

fn render_stroke<R: Rasterizer>(rast: &mut R, stroke: &VectorStroke, flattened: &Flattened) {
    if flattened.stroke_paths.is_empty() || stroke.width <= 0.0 || stroke.opa == 0 {
        return;
    }

    let mut bounds = flattened.bounds;
    let radius = stroke.width * 0.5;
    bounds.expand(radius + 1.5);

    let min_x = bounds.min_x.floor() as i32;
    let max_x = bounds.max_x.ceil() as i32;
    let min_y = bounds.min_y.floor() as i32;
    let max_y = bounds.max_y.ceil() as i32;

    let mut dirty_min_x = i32::MAX;
    let mut dirty_min_y = i32::MAX;
    let mut dirty_max_x = i32::MIN;
    let mut dirty_max_y = i32::MIN;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut coverage_sum = 0.0f32;
            let mut sample_hit = false;

            for oy in &SAMPLE_OFFSETS {
                for ox in &SAMPLE_OFFSETS {
                    let sample_point = FPoint::new(x as f32 + ox, y as f32 + oy);
                    if let Some(coverage) = stroke_sample_coverage(sample_point, stroke, &flattened.stroke_paths) {
                        coverage_sum += coverage;
                        sample_hit = true;
                    }
                }
            }

            if !sample_hit {
                continue;
            }

            let coverage = (coverage_sum * 255.0 / SAMPLE_COUNT as f32).round() as u32;
            if coverage == 0 {
                continue;
            }

            let sample_point = FPoint::new(x as f32 + 0.5, y as f32 + 0.5);
            let (base_color, grad_opa) = if let Some(grad) = &stroke.gradient {
                grad.gradient.color_at(sample_point)
            } else {
                (stroke.color.unwrap_or(Rgba8888::WHITE), OPA_COVER)
            };

            let mut final_opa = ((coverage as u32) * (stroke.opa as u32)) / 255;
            final_opa = (final_opa * grad_opa as u32) / 255;
            final_opa = final_opa.min(255);
            let final_opa = final_opa as Opa;
            if final_opa == 0 {
                continue;
            }

            rast.blend_pixel(x, y, base_color, final_opa);
            if x < dirty_min_x {
                dirty_min_x = x;
            }
            if x > dirty_max_x {
                dirty_max_x = x;
            }
            if y < dirty_min_y {
                dirty_min_y = y;
            }
            if y > dirty_max_y {
                dirty_max_y = y;
            }
        }
    }

    if dirty_min_x <= dirty_max_x && dirty_min_y <= dirty_max_y {
        rast.mark_dirty(dirty_min_x, dirty_min_y, dirty_max_x + 1, dirty_max_y + 1);
    }
}

fn stroke_sample_coverage(sample: FPoint, stroke: &VectorStroke, paths: &[StrokePath]) -> Option<f32> {
    let mut best_dist = f32::MAX;
    let mut best_path: Option<&StrokePath> = None;
    let mut best_along = 0.0;

    for path in paths {
        if path.segments.is_empty() {
            continue;
        }
        let (dist_sq, along, before_start, after_end) = closest_distance_sq(sample, path);
        if dist_sq < best_dist {
            best_dist = dist_sq;
            best_path = Some(path);
            best_along = if before_start {
                0.0
            } else if after_end {
                path.length
            } else {
                along
            };
        }
    }

    let path = best_path?;
    let dist = best_dist.sqrt();
    let radius = stroke.width * 0.5;
    let aa_width = 1.0;

    let mut inside = false;
    if path.length <= f32::EPSILON {
        inside = dist <= radius;
    } else {
        if stroke.dash_pattern.is_empty() {
            inside = dist <= radius + aa_width;
        } else if dist <= radius + aa_width {
            if dash_contains(best_along + stroke.dash_phase, path.length, &stroke.dash_pattern) {
                inside = true;
            } else {
                // Allow round caps to cover dash gaps near ends
                if matches!(stroke.cap, StrokeCap::Round) {
                    if best_along <= radius || (path.length - best_along) <= radius {
                        inside = true;
                    }
                }
            }
        }
    }

    if !inside {
        return None;
    }

    let coverage = if dist <= radius - aa_width {
        1.0
    } else if dist <= radius + aa_width {
        (radius + aa_width - dist) / (2.0 * aa_width)
    } else {
        0.0
    };

    if coverage <= 0.0 {
        None
    } else {
        Some(coverage.clamp(0.0, 1.0))
    }
}

fn closest_distance_sq(point: FPoint, path: &StrokePath) -> (f32, f32, bool, bool) {
    let mut best_dist_sq = f32::MAX;
    let mut best_along = 0.0;
    let mut before_start = false;
    let mut after_end = false;

    for segment in &path.segments {
        let (dist_sq, t, raw_t) = distance_sq_to_segment(point, segment.start, segment.end);
        if dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_along = segment.cumulative + (t * segment.length);
            before_start = raw_t < 0.0;
            after_end = raw_t > 1.0;
        }
    }

    if path.closed {
        before_start = false;
        after_end = false;
    }

    (best_dist_sq, best_along, before_start, after_end)
}

fn distance_sq_to_segment(point: FPoint, a: FPoint, b: FPoint) -> (f32, f32, f32) {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let apx = point.x - a.x;
    let apy = point.y - a.y;

    let len_sq = abx * abx + aby * aby;
    if len_sq <= f32::EPSILON {
        let dist_sq = apx * apx + apy * apy;
        return (dist_sq, 0.0, 0.0);
    }

    let raw_t = (apx * abx + apy * aby) / len_sq;
    let t = raw_t.clamp(0.0, 1.0);
    let proj_x = a.x + abx * t;
    let proj_y = a.y + aby * t;
    let dx = point.x - proj_x;
    let dy = point.y - proj_y;
    (dx * dx + dy * dy, t, raw_t)
}

fn dash_contains(mut pos: f32, total_len: f32, pattern: &[f32]) -> bool {
    if pattern.is_empty() || total_len <= f32::EPSILON {
        return true;
    }
    let mut pattern_len = 0.0;
    for &p in pattern {
        pattern_len += p.abs();
    }
    if pattern_len <= f32::EPSILON {
        return true;
    }
    if pos < 0.0 {
        pos = (pos % pattern_len) + pattern_len;
    }
    pos = pos % pattern_len;

    let mut accum = 0.0;
    for (idx, chunk) in pattern.iter().enumerate() {
        let chunk_len = chunk.abs();
        let next = accum + chunk_len;
        if pos < next {
            return idx % 2 == 0;
        }
        accum = next;
    }
    true
}

fn point_in_path(point: FPoint, edges: &[Edge], rule: FillRule) -> bool {
    match rule {
        FillRule::EvenOdd => point_even_odd(point, edges),
        FillRule::NonZero => point_non_zero(point, edges),
    }
}

fn point_even_odd(point: FPoint, edges: &[Edge]) -> bool {
    let mut inside = false;
    for edge in edges {
        let y1 = edge.start.y;
        let y2 = edge.end.y;
        let intersects = (y1 > point.y) != (y2 > point.y)
            && point.x
                < (edge.end.x - edge.start.x) * (point.y - y1) / (y2 - y1 + f32::EPSILON) + edge.start.x;
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn point_non_zero(point: FPoint, edges: &[Edge]) -> bool {
    let mut winding = 0;
    for edge in edges {
        if edge.start.y <= point.y {
            if edge.end.y > point.y && is_left(edge.start, edge.end, point) > 0.0 {
                winding += 1;
            }
        } else if edge.end.y <= point.y && is_left(edge.start, edge.end, point) < 0.0 {
            winding -= 1;
        }
    }
    winding != 0
}

fn is_left(a: FPoint, b: FPoint, p: FPoint) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}
