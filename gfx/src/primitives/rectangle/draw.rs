use alloc::vec;
use alloc::vec::Vec;

use crate::colors::Color;
use crate::primitives::features::fill::FillStyle;
use crate::primitives::features::gradient::{Gradient, GradientStop};
use crate::primitives::features::stroke::{StrokeColor, StrokeStyle};
use crate::primitives::rectangle::{CornerRadius, Rectangle};
use crate::rasterizer::RasterTarget;

use zeno::{Angle, ArcSize, ArcSweep, Bounds, Command, Fill as ZenoFill, Mask, Origin, Point, Vector};
use zeno::PathBuilder;

const TOP: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;

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

        if let Some(inner_bounds) = inner_bounds.as_ref() {
            if inner_bounds.width() > 0.0 && inner_bounds.height() > 0.0 {
                render_fill(self, canvas, inner_bounds, &inner_radii);
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
            );
        }
    }
}

fn render_fill(rect: &Rectangle<'_>, canvas: &mut dyn RasterTarget, bounds: &Bounds, radii: &[CornerRadius; 4]) {
    let path = build_round_rect_path(bounds, radii, true);
    if path.is_empty() {
        return;
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
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let width = (x1 - x0) as usize;
    let mut coverage_row = vec![0u8; width];
    let mut color_row: Vec<Color> = Vec::new();

    for row in 0..(y1 - y0) {
        let y = y0 + row;
        if y < clip.top || y >= clip.bottom {
            continue;
        }

        coverage_row.fill(0);
        let mut mask = Mask::new(&path);
        mask.style(ZenoFill::NonZero);
        mask.origin(Origin::TopLeft);
        mask.offset(Vector::new(-(x0 as f32), -((y0 + row) as f32)));
        mask.size(width as u32, 1);
        mask.render_into(&mut coverage_row, None);

        let start = clip.left.max(x0);
        let end = clip.right.min(x1);
        if start >= end {
            continue;
        }
        let start_idx = (start - x0) as usize;
        let end_idx = (end - x0) as usize;

        match &rect.fill {
            FillStyle::Solid(color) => {
                process_solid_row(canvas, color, y, x0, &coverage_row, start_idx, end_idx);
            }
            FillStyle::Gradient(gradient) => {
                let span_len = end_idx - start_idx;
                color_row.resize(span_len, Color::rgba(0, 0, 0, 0));
                for (i, idx) in (start_idx..end_idx).enumerate() {
                    if coverage_row[idx] == 0 {
                        continue;
                    }
                    let px = (x0 + idx as i32) as f32 + 0.5;
                    let py = y as f32 + 0.5;
                    color_row[i] = sample_gradient_color(gradient, px, py, bounds);
                }
                process_color_row(
                    canvas,
                    y,
                    x0,
                    &coverage_row,
                    start_idx,
                    end_idx,
                    &color_row,
                );
            }
        }
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
) {
    let path = build_border_path(&rect.area, outer_radii, inner_bounds, inner_radii);
    if path.is_empty() {
        return;
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
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let width = (x1 - x0) as usize;
    let mut coverage_row = vec![0u8; width];
    let mut color_row: Vec<Color> = Vec::new();

    for row in 0..(y1 - y0) {
        let y = y0 + row;
        if y < clip.top || y >= clip.bottom {
            continue;
        }

        coverage_row.fill(0);
        let mut mask = Mask::new(&path);
        mask.style(ZenoFill::NonZero);
        mask.origin(Origin::TopLeft);
        mask.offset(Vector::new(-(x0 as f32), -((y0 + row) as f32)));
        mask.size(width as u32, 1);
        mask.render_into(&mut coverage_row, None);

        let start = clip.left.max(x0);
        let end = clip.right.min(x1);
        if start >= end {
            continue;
        }
        let start_idx = (start - x0) as usize;
        let end_idx = (end - x0) as usize;
        let span_len = end_idx - start_idx;
        color_row.resize(span_len, Color::rgba(0, 0, 0, 0));

        for (i, idx) in (start_idx..end_idx).enumerate() {
            if coverage_row[idx] == 0 {
                continue;
            }
            let px = (x0 + idx as i32) as f32 + 0.5;
            let py = y as f32 + 0.5;
            let edge = classify_edge(px, py, &rect.area, edge_widths, edge_styles);
            let Some(edge_idx) = edge else {
                coverage_row[idx] = 0;
                continue;
            };
            let Some(style) = edge_styles[edge_idx] else {
                coverage_row[idx] = 0;
                continue;
            };
            color_row[i] = sample_stroke_color(&style.color, px, py, &rect.area);
        }

        process_color_row(
            canvas,
            y,
            x0,
            &coverage_row,
            start_idx,
            end_idx,
            &color_row,
        );
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
    let mut idx = start_idx;
    while idx < end_idx {
        while idx < end_idx && coverage[idx] == 0 {
            idx += 1;
        }
        if idx >= end_idx {
            break;
        }
        let run_start = idx;
        let mut opaque = true;
        while idx < end_idx && coverage[idx] > 0 {
            if coverage[idx] != 255 {
                opaque = false;
            }
            idx += 1;
        }
        let run_len = idx - run_start;
        let x_start = x0 + run_start as i32;
        if opaque {
            canvas.fill_solid_hspan(y as u16, x_start as u16, *color, run_len as u16);
        } else {
            canvas.blend_solid_hspan(
                y as u16,
                x_start as u16,
                *color,
                &coverage[run_start..run_start + run_len],
            );
        }
    }
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
    let mut idx = start_idx;
    while idx < end_idx {
        while idx < end_idx && coverage[idx] == 0 {
            idx += 1;
        }
        if idx >= end_idx {
            break;
        }
        let run_start = idx;
        while idx < end_idx && coverage[idx] > 0 {
            idx += 1;
        }
        let run_len = idx - run_start;
        let x_start = x0 + run_start as i32;
        let color_offset = run_start - start_idx;
        canvas.blend_color_hspan(
            y as u16,
            x_start as u16,
            &colors[color_offset..color_offset + run_len],
            &coverage[run_start..run_start + run_len],
        );
    }
}

fn sample_gradient_color<const GS: usize>(gradient: &Gradient<GS>, x: f32, y: f32, bounds: &Bounds) -> Color {
    match gradient {
        Gradient::Horizontal(stops) => {
            let width = bounds.width();
            let t = if width > 0.0 {
                ((x - bounds.min.x) / width * 255.0).clamp(0.0, 255.0)
            } else {
                0.0
            };
            sample_gradient_stop(stops, t as u8)
        }
        Gradient::Vertical(stops) => {
            let height = bounds.height();
            let t = if height > 0.0 {
                ((y - bounds.min.y) / height * 255.0).clamp(0.0, 255.0)
            } else {
                0.0
            };
            sample_gradient_stop(stops, t as u8)
        }
    }
}

fn sample_stroke_color<const GS: usize>(color: &StrokeColor<GS>, x: f32, y: f32, bounds: &Bounds) -> Color {
    match color {
        StrokeColor::Solid(c) => *c,
        StrokeColor::Gradient(gradient) => sample_gradient_color(gradient, x, y, bounds),
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

fn classify_edge<'a>(
    x: f32,
    y: f32,
    outer: &Bounds,
    widths: &[f32; 4],
    styles: &[Option<&'a StrokeStyle<'a, 2>>; 4],
) -> Option<usize> {
    let center_x = (outer.min.x + outer.max.x) * 0.5;
    let center_y = (outer.min.y + outer.max.y) * 0.5;
    let dx = x - center_x;
    let dy = y - center_y;

    let mut candidate = if dy.abs() >= dx.abs() {
        if dy < 0.0 { TOP } else { BOTTOM }
    } else if dx > 0.0 {
        RIGHT
    } else {
        LEFT
    };

    if !edge_available(candidate, widths, styles) {
        if let Some(fallback) = fallback_edge(x, y, outer, widths, styles) {
            candidate = fallback;
        } else {
            return None;
        }
    }
    Some(candidate)
}

fn edge_available<'a>(idx: usize, widths: &[f32; 4], styles: &[Option<&'a StrokeStyle<'a, 2>>; 4]) -> bool {
    widths[idx] > 0.0 && styles[idx].is_some()
}

fn fallback_edge<'a>(
    x: f32,
    y: f32,
    outer: &Bounds,
    widths: &[f32; 4],
    styles: &[Option<&'a StrokeStyle<'a, 2>>; 4],
) -> Option<usize> {
    let distances = [
        (y - outer.min.y).abs(),
        (outer.max.x - x).abs(),
        (outer.max.y - y).abs(),
        (x - outer.min.x).abs(),
    ];
    let mut best: Option<(usize, f32)> = None;
    for idx in 0..4 {
        if !edge_available(idx, widths, styles) {
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

fn build_round_rect_path(bounds: &Bounds, radii: &[CornerRadius; 4], clockwise: bool) -> Vec<Command> {
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

fn append_round_rect_clockwise(path: &mut Vec<Command>, bounds: &Bounds, radii: &[CornerRadius; 4]) {
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
    arc_or_line(path, tr.horizontal, tr.vertical, Point::new(x1, y0 + tr.vertical), ArcSweep::Positive);
    path.line_to((x1, y1 - br.vertical));
    arc_or_line(path, br.horizontal, br.vertical, Point::new(x1 - br.horizontal, y1), ArcSweep::Positive);
    path.line_to((x0 + bl.horizontal, y1));
    arc_or_line(path, bl.horizontal, bl.vertical, Point::new(x0, y1 - bl.vertical), ArcSweep::Positive);
    path.line_to((x0, y0 + tl.vertical));
    arc_or_line(path, tl.horizontal, tl.vertical, Point::new(x0 + tl.horizontal, y0), ArcSweep::Positive);
}

fn append_round_rect_counter_clockwise(path: &mut Vec<Command>, bounds: &Bounds, radii: &[CornerRadius; 4]) {
    let x0 = bounds.min.x;
    let y0 = bounds.min.y;
    let x1 = bounds.max.x;
    let y1 = bounds.max.y;
    let tl = radii[0];
    let tr = radii[1];
    let br = radii[2];
    let bl = radii[3];

    path.move_to((x0 + tl.horizontal, y0));
    arc_or_line(path, tl.horizontal, tl.vertical, Point::new(x0, y0 + tl.vertical), ArcSweep::Negative);
    path.line_to((x0, y1 - bl.vertical));
    arc_or_line(path, bl.horizontal, bl.vertical, Point::new(x0 + bl.horizontal, y1), ArcSweep::Negative);
    path.line_to((x1 - br.horizontal, y1));
    arc_or_line(path, br.horizontal, br.vertical, Point::new(x1, y1 - br.vertical), ArcSweep::Negative);
    path.line_to((x1, y0 + tr.vertical));
    arc_or_line(path, tr.horizontal, tr.vertical, Point::new(x1 - tr.horizontal, y0), ArcSweep::Negative);
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

    Some(Bounds::new(Point::new(left, top), Point::new(right, bottom)))
}

fn shape_clip(rect: &Rectangle<'_>, canvas: &dyn RasterTarget, bounds: &Bounds) -> Option<ClipRect> {
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
        Some(ClipRect { left, top, right, bottom })
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

