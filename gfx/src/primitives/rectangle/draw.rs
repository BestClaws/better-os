use alloc::vec::Vec;
use core::{
    array, mem,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::colors::Color;
use crate::primitives::features::fill::FillStyle;
use crate::primitives::features::gradient::{Gradient, GradientStop};
use crate::primitives::features::stroke::{StrokeColor, StrokeStyle};
use crate::primitives::rectangle::{CornerRadius, Rectangle};
use crate::rasterizer::RasterTarget;

use zeno::PathBuilder;
use zeno::{
    Angle, ArcSize, ArcSweep, Bounds, Command, Fill as ZenoFill, Mask, Origin, Point, Scratch,
    Vector,
};

const TOP: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;
const MAX_MASK_CHUNK_BYTES: usize = 4096;

#[derive(Clone, Copy)]
struct ClipRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl<'a> Rectangle<'a> {
    pub fn draw(&self, canvas: &mut dyn RasterTarget) {
        if self.area.is_empty() {
            return;
        }

        let mut outer_radii = self.corner_radii;
        let outer_width = self.area.width();
        let outer_height = self.area.height();
        if outer_width <= 0.0 || outer_height <= 0.0 {
            return;
        }

        normalize_corner_radii(&mut outer_radii, outer_width, outer_height);

        let edge_styles: [Option<&StrokeStyle<'a, 2>>; 4] = [
            self.edges[TOP].as_ref(),
            self.edges[RIGHT].as_ref(),
            self.edges[BOTTOM].as_ref(),
            self.edges[LEFT].as_ref(),
        ];
        let edge_widths = [
            edge_styles[TOP]
                .map(|s| s.stroke.width.max(0.0))
                .unwrap_or(0.0),
            edge_styles[RIGHT]
                .map(|s| s.stroke.width.max(0.0))
                .unwrap_or(0.0),
            edge_styles[BOTTOM]
                .map(|s| s.stroke.width.max(0.0))
                .unwrap_or(0.0),
            edge_styles[LEFT]
                .map(|s| s.stroke.width.max(0.0))
                .unwrap_or(0.0),
        ];

        let inner_radii = compute_inner_radii(&outer_radii, &edge_widths);
        let inner_bounds = compute_inner_bounds(&self.area, &edge_widths);

        let mut scratch = Scratch::new();

        if let Some(inner_bounds) = inner_bounds.as_ref() {
            if inner_bounds.width() > 0.0 && inner_bounds.height() > 0.0 {
                render_fill(self, canvas, inner_bounds, &inner_radii, &mut scratch);
            }
        }

        let has_border = edge_styles
            .iter()
            .zip(edge_widths.iter())
            .any(|(style, width)| style.is_some() && *width > 0.0);

        if has_border {
            render_border(
                self,
                canvas,
                &outer_radii,
                inner_bounds.as_ref(),
                &inner_radii,
                &edge_widths,
                &edge_styles,
                &mut scratch,
            );
        }
    }
}

fn render_fill(
    rect: &Rectangle<'_>,
    canvas: &mut dyn RasterTarget,
    bounds: &Bounds,
    radii: &[CornerRadius; 4],
    scratch: &mut Scratch,
) {
    let path = build_round_rect_path(bounds, radii, true);
    if path.is_empty() {
        return;
    }

    let path_bytes = path.capacity() * mem::size_of::<Command>();
    if path_bytes > 0 {
        record_temp_allocation(path_bytes);
    }

    let mask_bounds = zeno::bounds(&path[..], ZenoFill::NonZero, None);
    if mask_bounds.is_empty() {
        return;
    }

    let clip = match shape_clip(rect, canvas, &mask_bounds) {
        Some(clip) => clip,
        None => return,
    };

    let x0 = floor_i32(mask_bounds.min.x);
    let x1 = ceil_i32(mask_bounds.max.x);
    let y0 = floor_i32(mask_bounds.min.y);
    let y1 = ceil_i32(mask_bounds.max.y);
    let width = (x1 - x0).max(0) as usize;
    let height = (y1 - y0).max(0) as usize;
    if width == 0 || height == 0 {
        return;
    }

    let max_rows = (MAX_MASK_CHUNK_BYTES / width).max(1);
    let chunk_rows = max_rows.min(height);

    let mut mask_buffer: Vec<u8> = Vec::with_capacity(width * chunk_rows);
    let mut color_buf: Vec<Color> = Vec::with_capacity(width);
    record_temp_allocation(
        path_bytes + mask_buffer.capacity() + color_buf.capacity() * mem::size_of::<Color>(),
    );

    let fill_render = make_fill_render(&rect.fill, bounds);
    let start = clip.left.max(x0);
    let end = clip.right.min(x1);
    if start >= end {
        return;
    }
    let start_idx = (start - x0) as usize;
    let end_idx = (end - x0) as usize;
    if start_idx >= end_idx {
        return;
    }

    let mut chunk_start = 0;
    while chunk_start < height {
        let rows = chunk_rows.min(height - chunk_start);
        mask_buffer.resize(width * rows, 0);

        let chunk_top = y0 + chunk_start as i32;
        let mut mask = Mask::with_scratch(&path, scratch);
        mask.style(ZenoFill::NonZero);
        mask.origin(Origin::TopLeft);
        mask.size(width as u32, rows as u32);
        mask.offset(Vector::new(-(x0 as f32), -(chunk_top as f32)));
        mask.render_into(&mut mask_buffer, Some(width));

        let gradient_colors = match &fill_render {
            FillRender::Gradient(ctx) if ctx.axis() == GradientAxis::Horizontal => {
                let span_len = end_idx - start_idx;
                color_buf.clear();
                color_buf.reserve(span_len);
                // SAFETY: we reserve above and immediately write every element.
                unsafe {
                    color_buf.set_len(span_len);
                }
                let dest = color_buf.as_mut_ptr();
                let mut stepper = ctx.horizontal_stepper(x0, start_idx);
                unsafe {
                    for i in 0..span_len {
                        *dest.add(i) = stepper.sample_color();
                        stepper.advance();
                    }
                }
                Some(color_buf.as_slice())
            }
            _ => None,
        };

        for row in 0..rows {
            let row_idx = chunk_start + row;
            let y = y0 + row_idx as i32;
            if y < clip.top || y >= clip.bottom {
                continue;
            }

            let row_slice = &mask_buffer[row * width..(row + 1) * width];
            if !row_slice[start_idx..end_idx].iter().any(|&c| c != 0) {
                continue;
            }

            match &fill_render {
                FillRender::Solid(color) => {
                    process_solid_row(canvas, color, y, x0, row_slice, start_idx, end_idx);
                }
                FillRender::Gradient(ctx) => match ctx.axis() {
                    GradientAxis::Vertical => {
                        let color = ctx.sample_vertical_row(y);
                        process_solid_row(canvas, &color, y, x0, row_slice, start_idx, end_idx);
                    }
                    GradientAxis::Horizontal => {
                        let Some(colors) = gradient_colors else {
                            continue;
                        }; // defensive fallback
                        process_color_row(canvas, y, x0, row_slice, start_idx, end_idx, colors);
                    }
                },
            }
        }

        chunk_start += rows;
    }
}

fn render_border<'a>(
    rect: &Rectangle<'a>,
    canvas: &mut dyn RasterTarget,
    outer_radii: &[CornerRadius; 4],
    inner_bounds: Option<&Bounds>,
    inner_radii: &[CornerRadius; 4],
    edge_widths: &[f32; 4],
    edge_styles: &[Option<&'a StrokeStyle<'a, 2>>; 4],
    scratch: &mut Scratch,
) {
    let path = build_border_path(&rect.area, outer_radii, inner_bounds, inner_radii);
    if path.is_empty() {
        return;
    }

    let path_bytes = path.capacity() * mem::size_of::<Command>();
    if path_bytes > 0 {
        record_temp_allocation(path_bytes);
    }

    let mask_bounds = zeno::bounds(&path[..], ZenoFill::NonZero, None);
    if mask_bounds.is_empty() {
        return;
    }

    let clip = match shape_clip(rect, canvas, &mask_bounds) {
        Some(clip) => clip,
        None => return,
    };

    let x0 = floor_i32(mask_bounds.min.x);
    let x1 = ceil_i32(mask_bounds.max.x);
    let y0 = floor_i32(mask_bounds.min.y);
    let y1 = ceil_i32(mask_bounds.max.y);
    let width = (x1 - x0).max(0) as usize;
    let height = (y1 - y0).max(0) as usize;
    if width == 0 || height == 0 {
        return;
    }

    let start = clip.left.max(x0);
    let end = clip.right.min(x1);
    if start >= end {
        return;
    }
    let start_idx = (start - x0) as usize;
    let end_idx = (end - x0) as usize;
    if start_idx >= end_idx {
        return;
    }
    let span_len = end_idx - start_idx;
    let max_rows = (MAX_MASK_CHUNK_BYTES / width).max(1);
    let chunk_rows = max_rows.min(height);

    let mut mask_buffer: Vec<u8> = Vec::with_capacity(width * chunk_rows);
    let mut color_row: Vec<Color> = Vec::with_capacity(span_len);
    record_temp_allocation(
        path_bytes + mask_buffer.capacity() + span_len * mem::size_of::<Color>(),
    );
    let stroke_caches = build_stroke_caches(edge_styles, &rect.area);
    let classifier = EdgeClassifier::new(&rect.area, edge_widths, edge_styles);
    let step_fp = FIXED_ONE as i64;

    let mut chunk_start = 0;
    while chunk_start < height {
        let rows = chunk_rows.min(height - chunk_start);
        mask_buffer.resize(width * rows, 0);

        let chunk_top = y0 + chunk_start as i32;
        let mut mask = Mask::with_scratch(&path, scratch);
        mask.style(ZenoFill::NonZero);
        mask.origin(Origin::TopLeft);
        mask.size(width as u32, rows as u32);
        mask.offset(Vector::new(-(x0 as f32), -(chunk_top as f32)));
        mask.render_into(&mut mask_buffer, Some(width));

        for row in 0..rows {
            let row_idx = chunk_start + row;
            let y = y0 + row_idx as i32;
            if y < clip.top || y >= clip.bottom {
                continue;
            }

            let row_slice = &mut mask_buffer[row * width..(row + 1) * width];
            if !row_slice[start_idx..end_idx].iter().any(|&c| c != 0) {
                continue;
            }

            if color_row.len() < span_len {
                color_row.resize(span_len, Color::rgba(0, 0, 0, 0));
            } else {
                color_row.truncate(span_len);
            }

            // Fixed-point centers let us preserve the old classification behavior without per-pixel floats.
            let mut px_fp = pixel_center_fixed(x0 + start_idx as i32);
            let py_fp = pixel_center_fixed(y);

            for (i, idx) in (start_idx..end_idx).enumerate() {
                if row_slice[idx] == 0 {
                    px_fp += step_fp;
                    continue;
                }

                let edge = classifier.classify(px_fp, py_fp);
                let Some(edge_idx) = edge else {
                    row_slice[idx] = 0;
                    px_fp += step_fp;
                    continue;
                }; // drop coverage if no edge accepts the pixel

                let Some(color) = stroke_caches[edge_idx].sample(px_fp, py_fp) else {
                    row_slice[idx] = 0;
                    px_fp += step_fp;
                    continue;
                };

                color_row[i] = color;
                px_fp += step_fp;
            }

            process_color_row(canvas, y, x0, row_slice, start_idx, end_idx, &color_row);
        }

        chunk_start += rows;
    }
}

const FIXED_SHIFT: i32 = 16;
const FIXED_ONE: i32 = 1 << FIXED_SHIFT;
const FIXED_HALF: i32 = FIXED_ONE >> 1;
const GRADIENT_MAX: i32 = 255;

#[derive(Clone)]
struct GradientTable {
    colors: [Color; 256],
}

impl GradientTable {
    fn from_stops<const GS: usize>(stops: &GradientStop<GS>) -> Self {
        let colors = array::from_fn(|idx| sample_gradient_stop(stops, idx as u8));
        Self { colors }
    }

    #[inline(always)]
    fn color(&self, index: u8) -> Color {
        self.colors[index as usize]
    }
}

#[inline(always)]
fn to_fixed(value: f32) -> i32 {
    let scaled = value * FIXED_ONE as f32;
    if scaled >= 0.0 {
        (scaled + 0.5) as i32
    } else {
        (scaled - 0.5) as i32
    }
}

#[inline(always)]
fn pixel_center_fixed(value: i32) -> i64 {
    ((value as i64) << FIXED_SHIFT) + FIXED_HALF as i64
}

#[inline(always)]
fn clamp_gradient_value(value_q16: i64) -> u8 {
    let mut idx = (value_q16 >> FIXED_SHIFT) as i32;
    if idx < 0 {
        idx = 0;
    } else if idx > GRADIENT_MAX {
        idx = GRADIENT_MAX;
    }
    idx as u8
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GradientAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
struct LinearGradientContext {
    table: GradientTable,
    origin_fp: i32,
    step_q16: i32,
    axis: GradientAxis,
}

impl LinearGradientContext {
    fn horizontal<const GS: usize>(stops: &GradientStop<GS>, bounds: &Bounds) -> Self {
        let origin_fp = to_fixed(bounds.min.x);
        let width_fp = to_fixed(bounds.width());
        let step_q16 = if width_fp > 0 {
            (((GRADIENT_MAX as i64) << (FIXED_SHIFT * 2)) / width_fp as i64) as i32
        } else {
            0
        };
        Self {
            table: GradientTable::from_stops(stops),
            origin_fp,
            step_q16,
            axis: GradientAxis::Horizontal,
        }
    }

    fn vertical<const GS: usize>(stops: &GradientStop<GS>, bounds: &Bounds) -> Self {
        let origin_fp = to_fixed(bounds.min.y);
        let height_fp = to_fixed(bounds.height());
        let step_q16 = if height_fp > 0 {
            (((GRADIENT_MAX as i64) << (FIXED_SHIFT * 2)) / height_fp as i64) as i32
        } else {
            0
        };
        Self {
            table: GradientTable::from_stops(stops),
            origin_fp,
            step_q16,
            axis: GradientAxis::Vertical,
        }
    }

    #[inline(always)]
    fn axis(&self) -> GradientAxis {
        self.axis
    }

    #[inline(always)]
    fn horizontal_stepper(&self, x0: i32, start_idx: usize) -> GradientStepper<'_> {
        let pixel = x0 + start_idx as i32;
        let acc_q16 = self.acc_for(pixel_center_fixed(pixel));
        GradientStepper {
            table: &self.table,
            acc_q16,
            step_q16: self.step_q16,
        }
    }

    #[inline(always)]
    fn sample_vertical_row(&self, y: i32) -> Color {
        let acc_q16 = self.acc_for(pixel_center_fixed(y));
        self.table.color(clamp_gradient_value(acc_q16))
    }

    #[inline(always)]
    fn sample_fixed(&self, x_fp: i64, y_fp: i64) -> Color {
        let coord_fp = match self.axis {
            GradientAxis::Horizontal => x_fp,
            GradientAxis::Vertical => y_fp,
        };
        let acc_q16 = self.acc_for(coord_fp);
        self.table.color(clamp_gradient_value(acc_q16))
    }

    #[inline(always)]
    fn acc_for(&self, coord_fp: i64) -> i64 {
        let offset_fp = coord_fp - self.origin_fp as i64;
        (offset_fp * self.step_q16 as i64) >> FIXED_SHIFT
    }
}

enum FillRender {
    Solid(Color),
    Gradient(LinearGradientContext),
}

struct GradientStepper<'a> {
    table: &'a GradientTable,
    acc_q16: i64,
    step_q16: i32,
}

impl<'a> GradientStepper<'a> {
    #[inline(always)]
    fn sample_color(&self) -> Color {
        self.table.color(clamp_gradient_value(self.acc_q16))
    }

    #[inline(always)]
    fn advance(&mut self) {
        self.advance_by(1);
    }

    #[inline(always)]
    fn advance_by(&mut self, count: usize) {
        let delta = self.step_q16 as i64 * count as i64;
        self.acc_q16 += delta;
        let max = (GRADIENT_MAX as i64) << FIXED_SHIFT;
        if self.acc_q16 < 0 {
            self.acc_q16 = 0;
        } else if self.acc_q16 > max {
            self.acc_q16 = max;
        }
    }
}

#[derive(Clone)]
enum StrokeColorCache {
    None,
    Solid(Color),
    Gradient(LinearGradientContext),
}

impl StrokeColorCache {
    #[inline(always)]
    fn sample(&self, x_fp: i64, y_fp: i64) -> Option<Color> {
        match self {
            StrokeColorCache::None => None,
            StrokeColorCache::Solid(color) => Some(*color),
            StrokeColorCache::Gradient(ctx) => Some(ctx.sample_fixed(x_fp, y_fp)),
        }
    }
}

struct EdgeClassifier {
    center_x_fp: i64,
    center_y_fp: i64,
    outer_min_x_fp: i64,
    outer_max_x_fp: i64,
    outer_min_y_fp: i64,
    outer_max_y_fp: i64,
    available: [bool; 4],
}

impl EdgeClassifier {
    fn new(bounds: &Bounds, widths: &[f32; 4], styles: &[Option<&StrokeStyle<'_, 2>>; 4]) -> Self {
        let center_x = (bounds.min.x + bounds.max.x) * 0.5;
        let center_y = (bounds.min.y + bounds.max.y) * 0.5;
        let available = [
            widths[TOP] > 0.0 && styles[TOP].is_some(),
            widths[RIGHT] > 0.0 && styles[RIGHT].is_some(),
            widths[BOTTOM] > 0.0 && styles[BOTTOM].is_some(),
            widths[LEFT] > 0.0 && styles[LEFT].is_some(),
        ];

        Self {
            center_x_fp: to_fixed(center_x) as i64,
            center_y_fp: to_fixed(center_y) as i64,
            outer_min_x_fp: to_fixed(bounds.min.x) as i64,
            outer_max_x_fp: to_fixed(bounds.max.x) as i64,
            outer_min_y_fp: to_fixed(bounds.min.y) as i64,
            outer_max_y_fp: to_fixed(bounds.max.y) as i64,
            available,
        }
    }

    #[inline(always)]
    fn classify(&self, px_fp: i64, py_fp: i64) -> Option<usize> {
        let dx = px_fp - self.center_x_fp;
        let dy = py_fp - self.center_y_fp;
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        let mut candidate = if abs_dy >= abs_dx {
            if dy < 0 { TOP } else { BOTTOM }
        } else if dx > 0 {
            RIGHT
        } else {
            LEFT
        };

        if !self.available[candidate] {
            candidate = self.fallback(px_fp, py_fp)?;
        }

        Some(candidate)
    }

    #[inline(always)]
    fn fallback(&self, px_fp: i64, py_fp: i64) -> Option<usize> {
        let distances = [
            (py_fp - self.outer_min_y_fp).abs(),
            (self.outer_max_x_fp - px_fp).abs(),
            (self.outer_max_y_fp - py_fp).abs(),
            (px_fp - self.outer_min_x_fp).abs(),
        ];

        let mut best: Option<(usize, i64)> = None;
        for idx in 0..4 {
            if !self.available[idx] {
                continue;
            }
            let distance = distances[idx];
            match best {
                Some((_, best_dist)) if distance >= best_dist => {}
                _ => best = Some((idx, distance)),
            }
        }
        best.map(|(idx, _)| idx)
    }
}

fn make_linear_context<const GS: usize>(
    gradient: &Gradient<GS>,
    bounds: &Bounds,
) -> LinearGradientContext {
    match gradient {
        Gradient::Horizontal(stops) => LinearGradientContext::horizontal(stops, bounds),
        Gradient::Vertical(stops) => LinearGradientContext::vertical(stops, bounds),
    }
}

fn make_fill_render<const GS: usize>(fill: &FillStyle<GS>, bounds: &Bounds) -> FillRender {
    match fill {
        FillStyle::Solid(color) => FillRender::Solid(*color),
        FillStyle::Gradient(gradient) => {
            FillRender::Gradient(make_linear_context(gradient, bounds))
        }
    }
}

fn build_stroke_caches(
    styles: &[Option<&StrokeStyle<'_, 2>>; 4],
    bounds: &Bounds,
) -> [StrokeColorCache; 4] {
    [
        make_stroke_cache(styles[TOP], bounds),
        make_stroke_cache(styles[RIGHT], bounds),
        make_stroke_cache(styles[BOTTOM], bounds),
        make_stroke_cache(styles[LEFT], bounds),
    ]
}

fn make_stroke_cache(style: Option<&StrokeStyle<'_, 2>>, bounds: &Bounds) -> StrokeColorCache {
    match style {
        None => StrokeColorCache::None,
        Some(style) => match &style.color {
            StrokeColor::Solid(color) => StrokeColorCache::Solid(*color),
            StrokeColor::Gradient(gradient) => {
                StrokeColorCache::Gradient(make_linear_context(gradient, bounds))
            }
        },
    }
}

fn process_solid_row(
    canvas: &mut dyn RasterTarget,
    color: &Color,
    y: i32,
    x0: i32,
    coverage: &[u8],
    start_idx: usize,
    end_idx: usize,
) {
    if start_idx >= end_idx {
        return;
    }

    let span = &coverage[start_idx..end_idx];
    if span.is_empty() {
        return;
    }

    for_each_coverage_run(span, |offset, coverage, opaque| {
        let x_start = x0 + (start_idx + offset) as i32;
        if opaque {
            canvas.fill_solid_hspan(y as u16, x_start as u16, *color, coverage.len() as u16);
        } else {
            canvas.blend_solid_hspan(y as u16, x_start as u16, *color, coverage);
        }
    });
}

fn process_color_row(
    canvas: &mut dyn RasterTarget,
    y: i32,
    x0: i32,
    coverage: &[u8],
    start_idx: usize,
    end_idx: usize,
    colors: &[Color],
) {
    if start_idx >= end_idx {
        return;
    }

    let span = &coverage[start_idx..end_idx];
    if span.is_empty() {
        return;
    }

    debug_assert!(colors.len() >= span.len());

    for_each_coverage_run(span, |offset, coverage, _| {
        let x_start = x0 + (start_idx + offset) as i32;
        let color_slice = &colors[offset..offset + coverage.len()];
        canvas.blend_color_hspan(y as u16, x_start as u16, color_slice, coverage);
    });
}

#[inline(always)]
fn for_each_coverage_run(span: &[u8], mut visit: impl FnMut(usize, &[u8], bool)) {
    // Walk contiguous non-zero alpha runs once, collapsing repeated branching.
    let len = span.len();
    let mut idx = 0;

    while idx < len {
        while idx < len && span[idx] == 0 {
            idx += 1;
        }
        if idx >= len {
            break;
        }

        let start = idx;
        let mut opaque = true;
        while idx < len {
            let alpha = span[idx];
            if alpha == 0 {
                break;
            }
            if alpha != 255 {
                opaque = false;
            }
            idx += 1;
        }

        visit(start, &span[start..idx], opaque);
    }
}

fn sample_gradient_stop<const GS: usize>(stops: &GradientStop<GS>, position: u8) -> Color {
    let stops = &stops.0;
    if stops.is_empty() {
        return Color::rgba(0, 0, 0, 0);
    }
    if position <= stops[0].1 {
        return stops[0].0;
    }
    for window in stops.windows(2) {
        let (c0, p0) = (window[0].0, window[0].1);
        let (c1, p1) = (window[1].0, window[1].1);
        if position <= p1 {
            return lerp_color(c0, c1, p0, p1, position);
        }
    }
    stops[stops.len() - 1].0
}

fn lerp_color(c0: Color, c1: Color, p0: u8, p1: u8, position: u8) -> Color {
    if p1 == p0 {
        return c1;
    }
    let span = (p1 - p0) as u32;
    let offset = (position.saturating_sub(p0)) as u32;
    let r = lerp_channel(c0.r(), c1.r(), offset, span);
    let g = lerp_channel(c0.g(), c1.g(), offset, span);
    let b = lerp_channel(c0.b(), c1.b(), offset, span);
    let a = lerp_channel(c0.a(), c1.a(), offset, span);
    Color::rgba(r, g, b, a)
}

fn lerp_channel(a: u8, b: u8, offset: u32, span: u32) -> u8 {
    if span == 0 {
        return b;
    }
    let a = a as u32;
    let b = b as u32;
    let value = (a * (span - offset) + b * offset + (span / 2)) / span;
    value as u8
}

fn build_round_rect_path(
    bounds: &Bounds,
    radii: &[CornerRadius; 4],
    clockwise: bool,
) -> Vec<Command> {
    let mut path = Vec::new();
    if clockwise {
        append_round_rect_clockwise(&mut path, bounds, radii);
    } else {
        append_round_rect_counter_clockwise(&mut path, bounds, radii);
    }
    path.close();
    path
}

fn build_border_path(
    outer_bounds: &Bounds,
    outer_radii: &[CornerRadius; 4],
    inner_bounds: Option<&Bounds>,
    inner_radii: &[CornerRadius; 4],
) -> Vec<Command> {
    let mut path = Vec::new();
    append_round_rect_clockwise(&mut path, outer_bounds, outer_radii);
    path.close();
    if let Some(inner) = inner_bounds {
        if inner.width() > 0.0 && inner.height() > 0.0 {
            append_round_rect_counter_clockwise(&mut path, inner, inner_radii);
            path.close();
        }
    }
    path
}

fn append_round_rect_clockwise(
    path: &mut Vec<Command>,
    bounds: &Bounds,
    radii: &[CornerRadius; 4],
) {
    let x0 = bounds.min.x;
    let y0 = bounds.min.y;
    let x1 = bounds.max.x;
    let y1 = bounds.max.y;
    let tl = radii[0];
    let tr = radii[1];
    let br = radii[2];
    let bl = radii[3];

    path.move_to((x0 + tl.horizontal, y0));
    path.line_to((x1 - tr.horizontal, y0));
    arc_or_line(
        path,
        tr.horizontal,
        tr.vertical,
        Point::new(x1, y0 + tr.vertical),
        ArcSweep::Positive,
    );
    path.line_to((x1, y1 - br.vertical));
    arc_or_line(
        path,
        br.horizontal,
        br.vertical,
        Point::new(x1 - br.horizontal, y1),
        ArcSweep::Positive,
    );
    path.line_to((x0 + bl.horizontal, y1));
    arc_or_line(
        path,
        bl.horizontal,
        bl.vertical,
        Point::new(x0, y1 - bl.vertical),
        ArcSweep::Positive,
    );
    path.line_to((x0, y0 + tl.vertical));
    arc_or_line(
        path,
        tl.horizontal,
        tl.vertical,
        Point::new(x0 + tl.horizontal, y0),
        ArcSweep::Positive,
    );
}

fn append_round_rect_counter_clockwise(
    path: &mut Vec<Command>,
    bounds: &Bounds,
    radii: &[CornerRadius; 4],
) {
    let x0 = bounds.min.x;
    let y0 = bounds.min.y;
    let x1 = bounds.max.x;
    let y1 = bounds.max.y;
    let tl = radii[0];
    let tr = radii[1];
    let br = radii[2];
    let bl = radii[3];

    path.move_to((x0 + tl.horizontal, y0));
    arc_or_line(
        path,
        tl.horizontal,
        tl.vertical,
        Point::new(x0, y0 + tl.vertical),
        ArcSweep::Negative,
    );
    path.line_to((x0, y1 - bl.vertical));
    arc_or_line(
        path,
        bl.horizontal,
        bl.vertical,
        Point::new(x0 + bl.horizontal, y1),
        ArcSweep::Negative,
    );
    path.line_to((x1 - br.horizontal, y1));
    arc_or_line(
        path,
        br.horizontal,
        br.vertical,
        Point::new(x1, y1 - br.vertical),
        ArcSweep::Negative,
    );
    path.line_to((x1, y0 + tr.vertical));
    arc_or_line(
        path,
        tr.horizontal,
        tr.vertical,
        Point::new(x1 - tr.horizontal, y0),
        ArcSweep::Negative,
    );
}

fn arc_or_line(path: &mut Vec<Command>, rx: f32, ry: f32, to: Point, sweep: ArcSweep) {
    if rx > 0.0 && ry > 0.0 {
        path.arc_to(rx, ry, Angle::ZERO, ArcSize::Small, sweep, to);
    } else {
        path.line_to(to);
    }
}

fn normalize_corner_radii(radii: &mut [CornerRadius; 4], width: f32, height: f32) {
    for radius in radii.iter_mut() {
        if !radius.horizontal.is_finite() || radius.horizontal < 0.0 {
            radius.horizontal = 0.0;
        }
        if !radius.vertical.is_finite() || radius.vertical < 0.0 {
            radius.vertical = 0.0;
        }
    }

    loop {
        let mut adjusted = false;

        let top_sum = radii[0].horizontal + radii[1].horizontal;
        if top_sum > width && top_sum > 0.0 {
            let scale = width / top_sum;
            radii[0].horizontal *= scale;
            radii[1].horizontal *= scale;
            adjusted = true;
        }

        let bottom_sum = radii[3].horizontal + radii[2].horizontal;
        if bottom_sum > width && bottom_sum > 0.0 {
            let scale = width / bottom_sum;
            radii[3].horizontal *= scale;
            radii[2].horizontal *= scale;
            adjusted = true;
        }

        let left_sum = radii[0].vertical + radii[3].vertical;
        if left_sum > height && left_sum > 0.0 {
            let scale = height / left_sum;
            radii[0].vertical *= scale;
            radii[3].vertical *= scale;
            adjusted = true;
        }

        let right_sum = radii[1].vertical + radii[2].vertical;
        if right_sum > height && right_sum > 0.0 {
            let scale = height / right_sum;
            radii[1].vertical *= scale;
            radii[2].vertical *= scale;
            adjusted = true;
        }

        if !adjusted {
            break;
        }
    }
}

fn compute_inner_radii(outer: &[CornerRadius; 4], widths: &[f32; 4]) -> [CornerRadius; 4] {
    let mut inner = [CornerRadius::default(); 4];

    inner[0].horizontal = (outer[0].horizontal - widths[TOP]).max(0.0);
    inner[0].vertical = (outer[0].vertical - widths[LEFT]).max(0.0);

    inner[1].horizontal = (outer[1].horizontal - widths[TOP]).max(0.0);
    inner[1].vertical = (outer[1].vertical - widths[RIGHT]).max(0.0);

    inner[2].horizontal = (outer[2].horizontal - widths[BOTTOM]).max(0.0);
    inner[2].vertical = (outer[2].vertical - widths[RIGHT]).max(0.0);

    inner[3].horizontal = (outer[3].horizontal - widths[BOTTOM]).max(0.0);
    inner[3].vertical = (outer[3].vertical - widths[LEFT]).max(0.0);

    inner
}

fn compute_inner_bounds(outer: &Bounds, widths: &[f32; 4]) -> Option<Bounds> {
    let left = outer.min.x + widths[LEFT];
    let right = outer.max.x - widths[RIGHT];
    let top = outer.min.y + widths[TOP];
    let bottom = outer.max.y - widths[BOTTOM];

    if right <= left || bottom <= top {
        return None;
    }

    Some(Bounds::new(
        Point::new(left, top),
        Point::new(right, bottom),
    ))
}

fn shape_clip(
    rect: &Rectangle<'_>,
    canvas: &dyn RasterTarget,
    bounds: &Bounds,
) -> Option<ClipRect> {
    let area_left = floor_i32(bounds.min.x);
    let area_top = floor_i32(bounds.min.y);
    let area_right = ceil_i32(bounds.max.x);
    let area_bottom = ceil_i32(bounds.max.y);

    let clip_left = floor_i32(rect.clip.min.x);
    let clip_top = floor_i32(rect.clip.min.y);
    let clip_right = ceil_i32(rect.clip.max.x);
    let clip_bottom = ceil_i32(rect.clip.max.y);

    let canvas_right = canvas.width() as i32;
    let canvas_bottom = canvas.height() as i32;

    let left = area_left.max(clip_left).max(0);
    let top = area_top.max(clip_top).max(0);
    let right = area_right.min(clip_right).min(canvas_right);
    let bottom = area_bottom.min(clip_bottom).min(canvas_bottom);

    if left >= right || top >= bottom {
        None
    } else {
        Some(ClipRect {
            left,
            top,
            right,
            bottom,
        })
    }
}

fn floor_i32(value: f32) -> i32 {
    let mut n = value as i32;
    if (n as f32) > value {
        n -= 1;
    }
    n
}

fn ceil_i32(value: f32) -> i32 {
    let mut n = value as i32;
    if (n as f32) < value {
        n += 1;
    }
    n
}

static TEMP_ALLOCATION_PEAK: AtomicUsize = AtomicUsize::new(0);

fn record_temp_allocation(bytes: usize) {
    let mut peak = TEMP_ALLOCATION_PEAK.load(Ordering::Relaxed);
    while bytes > peak {
        match TEMP_ALLOCATION_PEAK.compare_exchange_weak(
            peak,
            bytes,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(updated) => peak = updated,
        }
    }
}

pub fn temp_allocation_peak_bytes() -> usize {
    TEMP_ALLOCATION_PEAK.load(Ordering::Relaxed)
}

pub fn reset_temp_allocation_peak() {
    TEMP_ALLOCATION_PEAK.store(0, Ordering::Relaxed);
}
