/// Line drawing matching LVGL's lv_draw_line

use crate::color::Rgba8888;
use crate::masks::{LineMask, LineSide, MaskRef, MaskResult};
use crate::primitives::common::{apply_scanline_masks, MaskBuffer};
use crate::primitives::rectangle::{draw_rect, RectDsc};
use crate::types::*;
use crate::Rasterizer;

fn pos_mod(value: i32, modulo: i32) -> i32 {
    let mut result = value % modulo;
    if result < 0 {
        result += modulo;
    }
    result
}

/// Line descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct LineDsc {
    pub p1: Point,
    pub p2: Point,
    pub width: i32,
    pub color: Rgba8888,
    pub opa: Opa,
    pub dash_width: i32,
    pub dash_gap: i32,
    pub round_start: bool,
    pub round_end: bool,
}

impl LineDsc {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self {
            p1,
            p2,
            width: 1,
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            dash_width: 0,
            dash_gap: 0,
            round_start: false,
            round_end: false,
        }
    }
}

/// Draw a line
pub fn draw_line<R>(rast: &mut R, dsc: &LineDsc)
where
    R: Rasterizer,
{
    let dx = dsc.p2.x - dsc.p1.x;
    let dy = dsc.p2.y - dsc.p1.y;

    // LVGL optimization: horizontal and vertical lines use rectangle drawing
    let is_horizontal = dy == 0 && dx != 0;
    let is_vertical = dx == 0 && dy != 0;
    let dashed = dsc.dash_width > 0 && dsc.dash_gap > 0;

    if dashed {
        if is_horizontal {
            draw_horizontal_dashed(rast, dsc);
            return;
        }

        if is_vertical {
            draw_vertical_dashed(rast, dsc);
            return;
        }
    }

    if (is_horizontal || is_vertical) && dsc.dash_width == 0 {
        draw_axis_aligned_solid(rast, dsc, is_horizontal);
        return;
    }

    // General case: diagonal line with mask-based rendering (matching LVGL)
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0 {
        // Point
        rast.blend_pixel(dsc.p1.x, dsc.p1.y, dsc.color, dsc.opa);
        return;
    }

    let geometry = LineGeometry::from_descriptor(dsc);
    render_line_with_masks(rast, dsc, &geometry);

    if dsc.round_start || dsc.round_end {
        let mut cap_dsc = RectDsc::new();
        cap_dsc.bg_color = dsc.color;
        cap_dsc.bg_opa = dsc.opa;
        cap_dsc.radius = RADIUS_CIRCLE;

        let radius = dsc.width >> 1;
        let r_corr = if dsc.width & 1 == 0 { 1 } else { 0 };

        if dsc.round_start && dsc.width > 0 {
            let area = Area::new(
                dsc.p1.x - radius,
                dsc.p1.y - radius,
                dsc.p1.x + radius - r_corr,
                dsc.p1.y + radius - r_corr,
            );
            draw_rect(rast, &cap_dsc, &area);
        }

        if dsc.round_end && dsc.width > 0 {
            let area = Area::new(
                dsc.p2.x - radius,
                dsc.p2.y - radius,
                dsc.p2.x + radius - r_corr,
                dsc.p2.y + radius - r_corr,
            );
            draw_rect(rast, &cap_dsc, &area);
        }
    }
}

fn draw_horizontal_dashed<R>(rast: &mut R, dsc: &LineDsc) -> Option<Area>
where
    R: Rasterizer,
{
    let w = dsc.width - 1;
    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    let x_start = dsc.p1.x.min(dsc.p2.x);
    let x_end = dsc.p1.x.max(dsc.p2.x) - 1;
    let y_start = dsc.p1.y - w_half1;
    let y_end = dsc.p1.y + w_half0;

    if x_end < x_start || y_end < y_start {
        return None;
    }

    let pattern = dsc.dash_width + dsc.dash_gap;
    if pattern <= 0 {
        return None;
    }

    let dash_start = pos_mod(x_start, pattern);
    let width = (x_end - x_start + 1) as usize;

    for y in y_start..=y_end {
        for offset in 0..width {
            let dash_cnt = (dash_start + offset as i32) % pattern;
            let x = x_start + offset as i32;

            if dash_cnt < dsc.dash_width {
                rast.blend_pixel(x, y, dsc.color, dsc.opa);
            } else {
                rast.stamp_rgb_zero_alpha(x, y, dsc.color);
            }
        }
    }
    Some(Area::new(x_start, y_start, x_end, y_end))
}

fn draw_vertical_dashed<R>(rast: &mut R, dsc: &LineDsc) -> Option<Area>
where
    R: Rasterizer,
{
    let w = dsc.width - 1;
    let w_half0 = w >> 1;
    let w_half1 = w_half0 + (w & 1);

    let x_start = dsc.p1.x - w_half1;
    let x_end = dsc.p1.x + w_half0;
    let y_start = dsc.p1.y.min(dsc.p2.y);
    let y_end = dsc.p1.y.max(dsc.p2.y) - 1;

    if x_end < x_start || y_end < y_start {
        return None;
    }

    let pattern = dsc.dash_width + dsc.dash_gap;
    if pattern <= 0 {
        return None;
    }

    let mut dash_cnt = pos_mod(y_start, pattern);

    for y in y_start..=y_end {
        if dash_cnt > dsc.dash_width {
            for x in x_start..=x_end {
                rast.stamp_rgb_zero_alpha(x, y, dsc.color);
            }
        } else {
            for x in x_start..=x_end {
                rast.blend_pixel(x, y, dsc.color, dsc.opa);
            }
        }

        if dash_cnt >= pattern {
            dash_cnt = 0;
        }
        dash_cnt += 1;
    }
    Some(Area::new(x_start, y_start, x_end, y_end))
}

/// Pre-computed masks and bounds for rasterising an anti-aliased line.
#[derive(Debug)]
struct LineGeometry {
    start: Point,
    end: Point,
    coverage_area: Area,
    mask_left: LineMask,
    mask_right: LineMask,
    mask_start: LineMask,
    mask_end: LineMask,
}

impl LineGeometry {
    /// Construct the mask set following LVGL's line drawing heuristics.
    fn from_descriptor(dsc: &LineDsc) -> Self {
        const WCORR: [u16; 33] = [
            128, 128, 128, 129, 129, 130, 130, 131, 132, 133, 134, 135, 137, 138, 140, 141, 143,
            145, 147, 149, 151, 153, 155, 158, 160, 162, 165, 167, 170, 173, 175, 178, 181,
        ];

        let xdiff = dsc.p2.x - dsc.p1.x;
        let ydiff = dsc.p2.y - dsc.p1.y;
        let flat = xdiff.abs() > ydiff.abs();

        let mut width = dsc.width;
        let wcorr_idx = if flat {
            ((ydiff.abs() << 5) / xdiff.abs()).min(32)
        } else {
            ((xdiff.abs() << 5) / ydiff.abs()).min(32)
        } as usize;
        width = ((width as i32 * WCORR[wcorr_idx] as i32 + 63) >> 7) as i32;

        let half_low = width >> 1;
        let half_high = half_low + (width & 1);

        let (start, end) = if flat {
            if xdiff > 0 {
                (dsc.p1, dsc.p2)
            } else {
                (dsc.p2, dsc.p1)
            }
        } else if ydiff > 0 {
            (dsc.p1, dsc.p2)
        } else {
            (dsc.p2, dsc.p1)
        };

        let (mask_left, mask_right) = if flat {
            let xdiff_ordered = end.x - start.x;
            if xdiff_ordered > 0 {
                (
                    LineMask::from_points(
                        Point::new(start.x, start.y - half_low),
                        Point::new(end.x, end.y - half_low),
                        LineSide::Left,
                    ),
                    LineMask::from_points(
                        Point::new(start.x, start.y + half_high),
                        Point::new(end.x, end.y + half_high),
                        LineSide::Right,
                    ),
                )
            } else {
                (
                    LineMask::from_points(
                        Point::new(start.x, start.y + half_high),
                        Point::new(end.x, end.y + half_high),
                        LineSide::Left,
                    ),
                    LineMask::from_points(
                        Point::new(start.x, start.y - half_low),
                        Point::new(end.x, end.y - half_low),
                        LineSide::Right,
                    ),
                )
            }
        } else {
            (
                LineMask::from_points(
                    Point::new(start.x + half_high, start.y),
                    Point::new(end.x + half_high, end.y),
                    LineSide::Left,
                ),
                LineMask::from_points(
                    Point::new(start.x - half_low, start.y),
                    Point::new(end.x - half_low, end.y),
                    LineSide::Right,
                ),
            )
        };

        let delta_y = end.y - start.y;
        let delta_x = end.x - start.x;
        let mask_start = LineMask::from_points(
            start,
            Point::new(start.x - delta_y, start.y + delta_x),
            LineSide::Bottom,
        );
        let mask_end = LineMask::from_points(
            end,
            Point::new(end.x - delta_y, end.y + delta_x),
            LineSide::Top,
        );

        let coverage_area = Area::new(
            start.x.min(end.x) - width,
            start.y.min(end.y) - width,
            start.x.max(end.x) + width,
            start.y.max(end.y) + width,
        );

        Self {
            start,
            end,
            coverage_area,
            mask_left,
            mask_right,
            mask_start,
            mask_end,
        }
    }

    fn mask_refs(&self) -> [MaskRef<'_>; 4] {
        [
            MaskRef::Line(&self.mask_left),
            MaskRef::Line(&self.mask_right),
            MaskRef::Line(&self.mask_start),
            MaskRef::Line(&self.mask_end),
        ]
    }
}

/// Render the diagonal line body using the prepared mask set.
fn render_line_with_masks<R>(rast: &mut R, dsc: &LineDsc, geometry: &LineGeometry)
where
    R: Rasterizer,
{
    let span_width = geometry.coverage_area.width().max(0) as usize;
    if span_width == 0 {
        return;
    }

    let mut mask_buffer = MaskBuffer::default();
    let mask_refs = geometry.mask_refs();

    for y in geometry.coverage_area.y1..=geometry.coverage_area.y2 {
        let (result, mask_values) = apply_scanline_masks(
            &mut mask_buffer,
            span_width,
            geometry.coverage_area.x1,
            y,
            &mask_refs,
        );

        if result == MaskResult::Transparent {
            continue;
        }

        let full_cover = result == MaskResult::FullCover
            && mask_values.iter().all(|&mask| mask == OPA_COVER);

        if full_cover {
            rast.blend_hspan_with(
                geometry.coverage_area.x1,
                y,
                geometry.coverage_area.width(),
                |_| (dsc.color, dsc.opa),
            );
            continue;
        }

        for (index, &mask) in mask_values.iter().enumerate() {
            let x = geometry.coverage_area.x1 + index as i32;
            let final_opa = if dsc.opa == OPA_COVER {
                mask
            } else {
                opa_mix(dsc.opa, mask)
            };
            rast.blend_pixel(x, y, dsc.color, final_opa);
        }
    }
}

/// Specialised fast-path for LVGL's horizontal/vertical solid lines.
fn draw_axis_aligned_solid<R>(rast: &mut R, dsc: &LineDsc, horizontal: bool) -> Option<Area>
where
    R: Rasterizer,
{
    let thickness = dsc.width - 1;
    let low_half = thickness / 2;
    let high_half = low_half + (thickness & 1);

    let (x1, x2, y1, y2) = if horizontal {
        (
            dsc.p1.x.min(dsc.p2.x),
            dsc.p1.x.max(dsc.p2.x) - 1,
            dsc.p1.y - high_half,
            dsc.p1.y + low_half,
        )
    } else {
        (
            dsc.p1.x - high_half,
            dsc.p1.x + low_half,
            dsc.p1.y.min(dsc.p2.y),
            dsc.p1.y.max(dsc.p2.y) - 1,
        )
    };

    if x2 < x1 || y2 < y1 {
        return None;
    }

    for y in y1..=y2 {
        rast.blend_hspan_with(x1, y, x2 - x1 + 1, |_| (dsc.color, dsc.opa));
    }

    Some(Area::new(x1, y1, x2, y2))
}
