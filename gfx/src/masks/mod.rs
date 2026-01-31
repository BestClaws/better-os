//! LVGL-compatible software rendering masks.
//! This module ports the relevant logic from lv_draw_sw_mask.c.

use alloc::vec;
use alloc::vec::Vec;
use core::cmp::{max, min};

use crate::math::{trigo_cos, trigo_sin};
use crate::types::{opa_mix, Area, Opa, Point};

/// Result of applying masks to a scanline.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MaskResult {
    Transparent,
    FullCover,
    Changed,
}

/// Angle mask descriptor (port of lv_draw_sw_mask_angle_param_t).
#[derive(Clone, Debug)]
pub struct AngleMask {
    vertex: Point,
    start_angle: i32,
    end_angle: i32,
    delta_deg: i32,
    start_line: LineMask,
    end_line: LineMask,
}

impl AngleMask {
    /// Create an angle mask keeping pixels between start and end degrees.
    pub fn new(vertex_x: i32, vertex_y: i32, start_angle: i32, end_angle: i32) -> Self {
        let vertex = Point::new(vertex_x, vertex_y);
        let start_angle = Self::normalize_angle(start_angle);
        let end_angle = Self::normalize_angle(end_angle);

        let delta_deg = if end_angle < start_angle {
            360 - start_angle + end_angle
        } else {
            (end_angle - start_angle).abs()
        };

        let start_side = if start_angle < 180 {
            LineSide::Left
        } else {
            LineSide::Right
        };
        let end_side = if end_angle < 180 {
            LineSide::Right
        } else {
            LineSide::Left
        };

        let start_line = LineMask::from_angle(vertex, start_angle, start_side);
        let end_line = LineMask::from_angle(vertex, end_angle, end_side);

        Self {
            vertex,
            start_angle,
            end_angle,
            delta_deg,
            start_line,
            end_line,
        }
    }

    fn normalize_angle(angle: i32) -> i32 {
        if angle < 0 {
            0
        } else if angle > 359 {
            359
        } else {
            angle
        }
    }

    fn adjust_cross(value: &mut i32, angle: i32) {
        if (angle > 270 && angle <= 359 && *value < 0)
            || (angle > 0 && angle <= 90 && *value < 0)
            || (angle > 90 && angle < 270 && *value > 0)
        {
            *value = 0;
        }
    }

    /// Apply angle mask to the provided buffer.
    pub fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        if mask_buf.is_empty() {
            return MaskResult::FullCover;
        }

        let len = mask_buf.len() as i32;
        let rel_y = abs_y - self.vertex.y;
        let rel_x = abs_x - self.vertex.x;

        if self.start_angle < 180
            && self.end_angle < 180
            && self.start_angle != 0
            && self.end_angle != 0
            && self.start_angle > self.end_angle
        {
            if abs_y < self.vertex.y {
                return MaskResult::FullCover;
            }

            let end_angle_first = ((rel_y as i64 * self.end_line.xy_steep as i64) >> 10) as i32;
            let mut start_angle_last =
                (((rel_y + 1) as i64 * self.start_line.xy_steep as i64) >> 10) as i32;

            Self::adjust_cross(&mut start_angle_last, self.start_angle);
            Self::adjust_cross(&mut start_angle_last, self.end_angle);

            let dist = (end_angle_first - start_angle_last) >> 1;
            let mut tmp = start_angle_last + dist - rel_x;
            if tmp > len {
                tmp = len;
            }

            let mut res1 = MaskResult::FullCover;
            if tmp > 0 {
                res1 = apply_line_segment(
                    &self.start_line,
                    &mut mask_buf[..tmp as usize],
                    abs_x,
                    abs_y,
                );
                if res1 == MaskResult::Transparent {
                    mask_buf[..tmp as usize].fill(0);
                }
            }

            if tmp > len {
                tmp = len;
            }
            if tmp < 0 {
                tmp = 0;
            }

            let mut res2 = MaskResult::FullCover;
            if tmp < len {
                res2 = apply_line_segment(
                    &self.end_line,
                    &mut mask_buf[tmp as usize..],
                    abs_x + tmp,
                    abs_y,
                );
                if res2 == MaskResult::Transparent {
                    mask_buf[tmp as usize..].fill(0);
                }
            }

            if res1 == res2 {
                res1
            } else {
                MaskResult::Changed
            }
        } else if self.start_angle > 180
            && self.end_angle > 180
            && self.start_angle > self.end_angle
        {
            if abs_y > self.vertex.y {
                return MaskResult::FullCover;
            }

            let end_angle_first = ((rel_y as i64 * self.end_line.xy_steep as i64) >> 10) as i32;
            let mut start_angle_last =
                (((rel_y + 1) as i64 * self.start_line.xy_steep as i64) >> 10) as i32;

            Self::adjust_cross(&mut start_angle_last, self.start_angle);
            Self::adjust_cross(&mut start_angle_last, self.end_angle);

            let dist = (end_angle_first - start_angle_last) >> 1;
            let mut tmp = start_angle_last + dist - rel_x;
            if tmp > len {
                tmp = len;
            }

            let mut res1 = MaskResult::FullCover;
            if tmp > 0 {
                res1 =
                    apply_line_segment(&self.end_line, &mut mask_buf[..tmp as usize], abs_x, abs_y);
                if res1 == MaskResult::Transparent {
                    mask_buf[..tmp as usize].fill(0);
                }
            }

            if tmp > len {
                tmp = len;
            }
            if tmp < 0 {
                tmp = 0;
            }

            let mut res2 = MaskResult::FullCover;
            if tmp < len {
                res2 = apply_line_segment(
                    &self.start_line,
                    &mut mask_buf[tmp as usize..],
                    abs_x + tmp,
                    abs_y,
                );
                if res2 == MaskResult::Transparent {
                    mask_buf[tmp as usize..].fill(0);
                }
            }

            if res1 == res2 {
                res1
            } else {
                MaskResult::Changed
            }
        } else {
            let mut res1 = MaskResult::FullCover;
            let mut res2 = MaskResult::FullCover;
            let mut res1_unknown = false;
            let mut res2_unknown = false;

            if self.start_angle == 180 {
                if abs_y < self.vertex.y {
                    res1 = MaskResult::FullCover;
                } else {
                    res1_unknown = true;
                }
            } else if self.start_angle == 0 {
                if abs_y < self.vertex.y {
                    res1_unknown = true;
                } else {
                    res1 = MaskResult::FullCover;
                }
            } else if (self.start_angle < 180 && abs_y < self.vertex.y)
                || (self.start_angle > 180 && abs_y >= self.vertex.y)
            {
                res1_unknown = true;
            } else {
                res1 = self.start_line.apply(mask_buf, abs_x, abs_y);
            }

            if self.end_angle == 180 {
                if abs_y < self.vertex.y {
                    res2_unknown = true;
                } else {
                    res2 = MaskResult::FullCover;
                }
            } else if self.end_angle == 0 {
                if abs_y < self.vertex.y {
                    res2 = MaskResult::FullCover;
                } else {
                    res2_unknown = true;
                }
            } else if (self.end_angle < 180 && abs_y < self.vertex.y)
                || (self.end_angle > 180 && abs_y >= self.vertex.y)
            {
                res2_unknown = true;
            } else {
                res2 = self.end_line.apply(mask_buf, abs_x, abs_y);
            }

            if res1 == MaskResult::Transparent || res2 == MaskResult::Transparent {
                MaskResult::Transparent
            } else if res1_unknown && res2_unknown {
                MaskResult::Transparent
            } else if !res1_unknown
                && !res2_unknown
                && res1 == MaskResult::FullCover
                && res2 == MaskResult::FullCover
            {
                MaskResult::FullCover
            } else {
                MaskResult::Changed
            }
        }
    }
}

/// Which side of the line to keep when masking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LineSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// Mask reference used when applying multiple masks.
pub enum MaskRef<'a> {
    Line(&'a LineMask),
    Angle(&'a AngleMask),
    Radius(&'a RadiusMask),
}

/// Software line mask descriptor (port of lv_draw_sw_mask_line_param_t).
#[derive(Clone, Debug)]
pub struct LineMask {
    p1: Point,
    p2: Point,
    side: LineSide,
    origo: Point,
    xy_steep: i32,
    yx_steep: i32,
    steep: i32,
    spx: i32,
    flat: bool,
    inv: bool,
}

impl LineMask {
    /// Create mask from two points keeping the specified side.
    pub fn from_points(mut p1: Point, mut p2: Point, side: LineSide) -> Self {
        if p1.y == p2.y && matches!(side, LineSide::Bottom) {
            p1.y -= 1;
            p2.y -= 1;
        }

        if p1.y > p2.y {
            core::mem::swap(&mut p1, &mut p2);
        }

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let flat = dx.abs() > dy.abs();

        let mut xy_steep = 0;
        let mut yx_steep = 0;

        if flat {
            if dx != 0 {
                let m = ((1i64 << 20) / dx as i64) as i64;
                yx_steep = ((m * dy as i64) >> 10) as i32;
            }
            if dy != 0 {
                let m = ((1i64 << 20) / dy as i64) as i64;
                xy_steep = ((m * dx as i64) >> 10) as i32;
            }
        } else {
            if dy != 0 {
                let m = ((1i64 << 20) / dy as i64) as i64;
                xy_steep = ((m * dx as i64) >> 10) as i32;
            }
            if dx != 0 {
                let m = ((1i64 << 20) / dx as i64) as i64;
                yx_steep = ((m * dy as i64) >> 10) as i32;
            }
        }

        let steep = if flat { yx_steep } else { xy_steep };
        let mut inv = false;

        match side {
            LineSide::Left => inv = false,
            LineSide::Right => inv = true,
            LineSide::Top => inv = steep > 0,
            LineSide::Bottom => inv = steep <= 0,
        }

        let mut spx = steep >> 2;
        if steep < 0 {
            spx = -spx;
        }

        Self {
            p1,
            p2,
            side,
            origo: p1,
            xy_steep,
            yx_steep,
            steep,
            spx,
            flat,
            inv,
        }
    }

    /// Create mask from vertex and angle in degrees.
    pub fn from_angle(p: Point, mut angle: i32, side: LineSide) -> Self {
        if angle > 180 {
            angle -= 180;
        }
        let p2x = (trigo_cos(angle) >> 5) + p.x;
        let p2y = (trigo_sin(angle) >> 5) + p.y;
        Self::from_points(p, Point::new(p2x, p2y), side)
    }

    fn apply_flat(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;
        let mut y_at_x = ((self.yx_steep as i64 * abs_x as i64) >> 10) as i32;

        if self.yx_steep > 0 {
            if y_at_x > abs_y {
                return if self.inv {
                    MaskResult::FullCover
                } else {
                    MaskResult::Transparent
                };
            }
        } else if y_at_x < abs_y {
            return if self.inv {
                MaskResult::FullCover
            } else {
                MaskResult::Transparent
            };
        }

        y_at_x = ((self.yx_steep as i64 * (abs_x + len) as i64) >> 10) as i32;
        if self.yx_steep > 0 {
            if y_at_x < abs_y {
                return if self.inv {
                    MaskResult::Transparent
                } else {
                    MaskResult::FullCover
                };
            }
        } else if y_at_x > abs_y {
            return if self.inv {
                MaskResult::Transparent
            } else {
                MaskResult::FullCover
            };
        }

        let xe = if self.yx_steep > 0 {
            ((abs_y * 256) as i64 * self.xy_steep as i64) >> 10
        } else {
            (((abs_y + 1) * 256) as i64 * self.xy_steep as i64) >> 10
        };

        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;
        let mut px_h = if xef == 0 {
            255
        } else {
            255 - (((255 - xef) * self.spx) >> 8)
        };
        let mut k = xei - abs_x;

        if xef != 0 {
            if k >= 0 && k < len {
                let mut m = 255 - (((255 - xef) * (255 - px_h)) >> 9);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            k += 1;
        }

        while px_h > self.spx {
            if k >= 0 && k < len {
                let mut m = px_h - (self.spx >> 1);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            px_h -= self.spx;
            k += 1;
            if k >= len {
                break;
            }
        }

        if k < len && k >= 0 {
            let x_inters = ((px_h as i64 * self.xy_steep as i64) >> 10) as i32;
            let mut m = (x_inters * px_h) >> 9;
            if self.yx_steep < 0 {
                m = 255 - m;
            }
            if self.inv {
                m = 255 - m;
            }
            mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
        }

        if self.inv {
            let mut k = xei - abs_x;
            if k > len {
                return MaskResult::Transparent;
            }
            if k > 0 {
                mask_buf[..k as usize].fill(0);
            }
        } else {
            k += 1;
            if k < 0 {
                return MaskResult::Transparent;
            }
            if k < len {
                mask_buf[k as usize..].fill(0);
            }
        }

        MaskResult::Changed
    }

    fn apply_steep(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;
        let mut x_at_y = ((self.xy_steep as i64 * abs_y as i64) >> 10) as i32;
        if self.xy_steep > 0 {
            x_at_y += 1;
        }
        if x_at_y < abs_x {
            return if self.inv {
                MaskResult::FullCover
            } else {
                MaskResult::Transparent
            };
        }

        x_at_y = ((self.xy_steep as i64 * abs_y as i64) >> 10) as i32;
        if x_at_y > abs_x + len {
            return if self.inv {
                MaskResult::Transparent
            } else {
                MaskResult::FullCover
            };
        }

        let xs = ((abs_y * 256) as i64 * self.xy_steep as i64) >> 10;
        let mut xsi = (xs >> 8) as i32;
        let mut xsf = (xs & 0xFF) as i32;

        let xe = (((abs_y + 1) * 256) as i64 * self.xy_steep as i64) >> 10;
        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;

        let mut k = xsi - abs_x;
        if xsi != xei && self.xy_steep < 0 && xsf == 0 {
            xsf = 0xFF;
            xsi = xei;
            k -= 1;
        }

        if xsi == xei {
            if k >= 0 && k < len {
                let mut m = (xsf + xef) >> 1;
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            k += 1;

            if self.inv {
                let mut k = xsi - abs_x;
                if k >= len {
                    return MaskResult::Transparent;
                }
                if k > 0 {
                    mask_buf[..k as usize].fill(0);
                }
            } else {
                if k > len {
                    k = len;
                }
                if k == 0 {
                    return MaskResult::Transparent;
                }
                if k < len {
                    mask_buf[k as usize..].fill(0);
                }
            }
        } else {
            if self.xy_steep < 0 {
                let y_inters = ((xsf * (-self.yx_steep)) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = (y_inters * xsf) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k -= 1;

                let x_inters = (((255 - y_inters) * (-self.xy_steep)) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = 255 - (((255 - y_inters) * x_inters) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 2;

                if self.inv {
                    let mut k = xsi - abs_x - 1;
                    if k > len {
                        k = len;
                    }
                    if k > 0 {
                        mask_buf[..k as usize].fill(0);
                    }
                } else {
                    if k > len {
                        return MaskResult::FullCover;
                    }
                    if k >= 0 {
                        mask_buf[k as usize..].fill(0);
                    }
                }
            } else {
                let y_inters = (((255 - xsf) * self.yx_steep) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = 255 - ((y_inters * (255 - xsf)) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 1;

                let x_inters = (((255 - y_inters) * self.xy_steep) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = ((255 - y_inters) * x_inters) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 1;

                if self.inv {
                    let mut k = xsi - abs_x;
                    if k > len {
                        return MaskResult::Transparent;
                    }
                    if k > 0 {
                        mask_buf[..k as usize].fill(0);
                    }
                } else {
                    if k > len {
                        k = len;
                    }
                    if k == 0 {
                        return MaskResult::Transparent;
                    }
                    if k < len {
                        mask_buf[k as usize..].fill(0);
                    }
                }
            }
        }

        MaskResult::Changed
    }

    /// Apply line mask to a scanline buffer.
    pub fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;

        let rel_y = abs_y - self.origo.y;
        let rel_x = abs_x - self.origo.x;

        if self.steep == 0 {
            if self.flat {
                match self.side {
                    LineSide::Left | LineSide::Right => return MaskResult::FullCover,
                    LineSide::Top if rel_y < 0 => return MaskResult::FullCover,
                    LineSide::Bottom if rel_y > 0 => return MaskResult::FullCover,
                    _ => return MaskResult::Transparent,
                }
            } else {
                match self.side {
                    LineSide::Top | LineSide::Bottom => return MaskResult::FullCover,
                    LineSide::Right if rel_x > 0 => return MaskResult::FullCover,
                    LineSide::Left => {
                        if rel_x + len < 0 {
                            return MaskResult::FullCover;
                        }
                        let k = -rel_x;
                        if k < 0 {
                            return MaskResult::Transparent;
                        }
                        if k < len {
                            mask_buf[k as usize..].fill(0);
                            return MaskResult::Changed;
                        }
                        return MaskResult::Changed;
                    }
                    LineSide::Right => {
                        if rel_x + len < 0 {
                            return MaskResult::Transparent;
                        }
                        let mut k = -rel_x;
                        if k < 0 {
                            k = 0;
                        }
                        if k >= len {
                            return MaskResult::Transparent;
                        }
                        mask_buf[..k as usize].fill(0);
                        return MaskResult::Changed;
                    }
                    _ => return MaskResult::FullCover,
                }
            }
        }

        if self.flat {
            self.apply_flat(mask_buf, rel_x, rel_y)
        } else {
            self.apply_steep(mask_buf, rel_x, rel_y)
        }
    }
}

/// Cached data for rounded rectangle masks.
#[derive(Clone, Debug)]
struct RadiusCircle {
    radius: i32,
    cir_opa: Vec<Opa>,
    opa_start_on_y: Vec<usize>,
    x_start_on_y: Vec<i32>,
}

impl RadiusCircle {
    fn new(radius: i32) -> Self {
        let mut circle = RadiusCircle {
            radius,
            cir_opa: Vec::new(),
            opa_start_on_y: Vec::new(),
            x_start_on_y: Vec::new(),
        };
        circle.calc_aa4();
        circle
    }

    fn calc_aa4(&mut self) {
        if self.radius <= 0 {
            self.cir_opa.clear();
            self.opa_start_on_y.clear();
            self.x_start_on_y.clear();
            return;
        }

        if self.radius == 1 {
            self.cir_opa = vec![180, 0];
            self.opa_start_on_y = vec![0, 1, 2];
            self.x_start_on_y = vec![0, 0];
            return;
        }

        let radius = self.radius;
        let cir_capacity = ((radius + 1) * 2 * 2) as usize;
        let mut cir_x = vec![0i32; cir_capacity];
        let mut cir_y = vec![0i32; cir_capacity];
        let mut cir_opa_vals = vec![0i32; (radius * 6 + 6) as usize];
        let mut cir_size: usize = 0;

        let mut cp_x = radius * 4;
        let mut cp_y = 0;
        let mut tmp = 1 - cp_x;

        let mut y_8th_cnt = 0i32;
        let mut x_int = [0i32; 4];
        let mut x_fract = [0i32; 4];
        x_int[0] = cp_x >> 2;
        x_fract[0] = 0;

        while cp_y <= cp_x {
            let mut i = 0;
            while i < 4 {
                if tmp <= 0 {
                    tmp += 2 * cp_y + 3;
                } else {
                    tmp += 2 * (cp_y - cp_x) + 5;
                    cp_x -= 1;
                }
                cp_y += 1;

                if cp_y > cp_x {
                    break;
                }

                x_int[i] = cp_x >> 2;
                x_fract[i] = cp_x & 0x3;
                i += 1;
            }

            if i != 4 {
                break;
            }

            let mut push_entry = |vx: i32, vy: i32, opa: i32, cir_size: &mut usize| {
                cir_x[*cir_size] = vx;
                cir_y[*cir_size] = vy;
                cir_opa_vals[*cir_size] = opa * 16;
                *cir_size += 1;
            };

            if x_int[0] == x_int[3] {
                push_entry(
                    x_int[0],
                    y_8th_cnt,
                    x_fract[0] + x_fract[1] + x_fract[2] + x_fract[3],
                    &mut cir_size,
                );
            } else if x_int[0] != x_int[1] {
                push_entry(x_int[0], y_8th_cnt, x_fract[0], &mut cir_size);
                push_entry(
                    x_int[0] - 1,
                    y_8th_cnt,
                    4 + x_fract[1] + x_fract[2] + x_fract[3],
                    &mut cir_size,
                );
            } else if x_int[0] != x_int[2] {
                push_entry(x_int[0], y_8th_cnt, x_fract[0] + x_fract[1], &mut cir_size);
                push_entry(
                    x_int[0] - 1,
                    y_8th_cnt,
                    8 + x_fract[2] + x_fract[3],
                    &mut cir_size,
                );
            } else {
                push_entry(
                    x_int[0],
                    y_8th_cnt,
                    x_fract[0] + x_fract[1] + x_fract[2],
                    &mut cir_size,
                );
                push_entry(x_int[0] - 1, y_8th_cnt, 12 + x_fract[3], &mut cir_size);
            }

            y_8th_cnt += 1;
        }

        let mid = radius * 723;
        let mid_int = mid >> 10;
        if cir_size == 0 || cir_x[cir_size - 1] != mid_int || cir_y[cir_size - 1] != mid_int {
            let mut tmp_val = mid - (mid_int << 10);
            if tmp_val <= 512 {
                tmp_val = (tmp_val * tmp_val * 2) >> (10 + 6);
            } else {
                tmp_val = 1024 - tmp_val;
                tmp_val = (tmp_val * tmp_val * 2) >> (10 + 6);
                tmp_val = 15 - tmp_val;
            }

            cir_x[cir_size] = mid_int;
            cir_y[cir_size] = mid_int;
            cir_opa_vals[cir_size] = tmp_val * 16;
            cir_size += 1;
        }

        if cir_size >= 2 {
            let mut i = cir_size as isize - 2;
            while i >= 0 {
                let idx = i as usize;
                cir_x[cir_size] = cir_y[idx];
                cir_y[cir_size] = cir_x[idx];
                cir_opa_vals[cir_size] = cir_opa_vals[idx];
                cir_size += 1;
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }

        self.cir_opa.clear();
        self.x_start_on_y.clear();
        self.opa_start_on_y.clear();

        let mut i = 0usize;
        let mut y = 0i32;
        while i < cir_size {
            self.opa_start_on_y.push(self.cir_opa.len());
            let mut min_x = cir_x[i];
            while i < cir_size && cir_y[i] == y {
                min_x = min(min_x, cir_x[i]);
                self.cir_opa.push(cir_opa_vals[i].min(255) as Opa);
                i += 1;
            }
            self.x_start_on_y.push(min_x);
            y += 1;
        }

        self.opa_start_on_y.push(self.cir_opa.len());
    }

    fn get_line(&self, y: i32) -> Option<(usize, i32, &[Opa])> {
        if y < 0 || y + 1 >= self.opa_start_on_y.len() as i32 {
            return None;
        }
        let idx = y as usize;
        let start = self.opa_start_on_y[idx];
        let end = self.opa_start_on_y[idx + 1];
        let x_start = self.x_start_on_y[idx];
        Some((end - start, x_start, &self.cir_opa[start..end]))
    }
}

/// Rounded rectangle radius mask descriptor (port of lv_draw_sw_mask_radius_param_t).
#[derive(Clone, Debug)]
pub struct RadiusMask {
    rect: Area,
    radius: i32,
    outer: bool,
    circle: Option<RadiusCircle>,
}

impl RadiusMask {
    /// Create a radius mask for the given rectangle.
    pub fn new(rect: Area, mut radius: i32, outer: bool) -> Self {
        let w = rect.width();
        let h = rect.height();
        let short_side = min(w, h);
        if radius > short_side / 2 {
            radius = short_side / 2;
        }
        if radius < 0 {
            radius = 0;
        }

        let circle = if radius > 0 {
            Some(RadiusCircle::new(radius))
        } else {
            None
        };

        Self {
            rect,
            radius,
            outer,
            circle,
        }
    }

    /// Apply radius mask to the provided buffer.
    pub fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;
        let rect = self.rect;

        if !self.outer {
            if abs_y < rect.y1 || abs_y > rect.y2 {
                return MaskResult::Transparent;
            }
        } else if abs_y < rect.y1 || abs_y > rect.y2 {
            return MaskResult::FullCover;
        }

        if (abs_x >= rect.x1 + self.radius && abs_x + len <= rect.x2 - self.radius)
            || (abs_y >= rect.y1 + self.radius && abs_y <= rect.y2 - self.radius)
        {
            if !self.outer {
                let last = rect.x1 - abs_x;
                if last > len {
                    return MaskResult::Transparent;
                }
                if last > 0 {
                    mask_buf[..last as usize].fill(0);
                }
                let first = rect.x2 - abs_x + 1;
                if first <= 0 {
                    return MaskResult::Transparent;
                }
                if first < len {
                    mask_buf[first as usize..].fill(0);
                }
                if last == 0 && first == len {
                    return MaskResult::FullCover;
                }
                return MaskResult::Changed;
            } else {
                let mut first = rect.x1 - abs_x;
                if first < 0 {
                    first = 0;
                }
                if first <= len {
                    let mut last = rect.x2 - abs_x - first + 1;
                    if first + last > len {
                        last = len - first;
                    }
                    if last > 0 {
                        mask_buf[first as usize..(first + last) as usize].fill(0);
                    }
                }
                return MaskResult::Changed;
            }
        }

        let circle = match &self.circle {
            Some(c) => c,
            None => return MaskResult::Changed,
        };

        let k = rect.x1 - abs_x;
        let w = rect.width();
        let h = rect.height();
        let rel_x = abs_x - rect.x1;
        let rel_y = abs_y - rect.y1;

        let cir_y = if rel_y < self.radius {
            self.radius - rel_y - 1
        } else {
            rel_y - (h - self.radius)
        };

        let (aa_len, x_start, opa_slice) = match circle.get_line(cir_y) {
            Some(v) => v,
            None => return MaskResult::Changed,
        };

        let cir_x_right = k + w - self.radius + x_start;
        let cir_x_left = k + self.radius - x_start - 1;

        if !self.outer {
            for idx in 0..aa_len {
                let opa = opa_slice[aa_len - idx - 1];
                let right_idx = cir_x_right + idx as i32;
                if right_idx >= 0 && right_idx < len {
                    let buf_idx = right_idx as usize;
                    mask_buf[buf_idx] = mask_mix(mask_buf[buf_idx], opa);
                }
                let left_idx = cir_x_left - idx as i32;
                if left_idx >= 0 && left_idx < len {
                    let buf_idx = left_idx as usize;
                    mask_buf[buf_idx] = mask_mix(mask_buf[buf_idx], opa);
                }
            }

            let right_clean = clamp_i32(0, cir_x_right + aa_len as i32, len);
            if right_clean < len {
                mask_buf[right_clean as usize..].fill(0);
            }

            let left_clean = clamp_i32(0, cir_x_left - aa_len as i32 + 1, len);
            if left_clean > 0 {
                mask_buf[..left_clean as usize].fill(0);
            }
        } else {
            for idx in 0..aa_len {
                let opa_val = 255 - opa_slice[aa_len - idx - 1];
                let right_idx = cir_x_right + idx as i32;
                if right_idx >= 0 && right_idx < len {
                    let buf_idx = right_idx as usize;
                    mask_buf[buf_idx] = mask_mix(mask_buf[buf_idx], opa_val);
                }
                let left_idx = cir_x_left - idx as i32;
                if left_idx >= 0 && left_idx < len {
                    let buf_idx = left_idx as usize;
                    mask_buf[buf_idx] = mask_mix(mask_buf[buf_idx], opa_val);
                }
            }

            let clr_start = clamp_i32(0, cir_x_left + 1, len);
            let clr_len = clamp_i32(0, cir_x_right - clr_start, len - clr_start);
            if clr_len > 0 {
                mask_buf[clr_start as usize..(clr_start + clr_len) as usize].fill(0);
            }
        }

        MaskResult::Changed
    }

    #[cfg(debug_assertions)]
    pub fn debug_line_info(&self, rel_y: i32) -> Option<(i32, Vec<Opa>)> {
        let circle = self.circle.as_ref()?;
        let h = self.rect.height();
        let cir_y = if rel_y < self.radius {
            self.radius - rel_y - 1
        } else {
            rel_y - (h - self.radius)
        };
        let (len, x_start, slice) = circle.get_line(cir_y)?;
        Some((x_start, slice[..len].to_vec()))
    }
}

/// Apply multiple masks to a scanline buffer.
pub fn apply_masks(
    masks: &[MaskRef<'_>],
    mask_buf: &mut [Opa],
    abs_x: i32,
    abs_y: i32,
) -> MaskResult {
    if masks.is_empty() {
        return MaskResult::FullCover;
    }

    let mut changed = false;
    for mask in masks {
        let res = match mask {
            MaskRef::Line(line) => line.apply(mask_buf, abs_x, abs_y),
            MaskRef::Angle(angle) => angle.apply(mask_buf, abs_x, abs_y),
            MaskRef::Radius(radius) => radius.apply(mask_buf, abs_x, abs_y),
        };
        match res {
            MaskResult::Transparent => return MaskResult::Transparent,
            MaskResult::FullCover => {}
            MaskResult::Changed => changed = true,
        }
    }

    if changed {
        MaskResult::Changed
    } else {
        MaskResult::FullCover
    }
}

#[inline]
fn mask_mix(mask_act: Opa, mask_new: Opa) -> Opa {
    if mask_act >= 255 {
        return mask_new;
    }
    if mask_act == 0 {
        return 0;
    }
    let prod = (mask_act as u32) * (mask_new as u32);
    ((prod * 0x8081) >> 23) as Opa
}

#[inline]
fn clamp_i32(min_v: i32, val: i32, max_v: i32) -> i32 {
    min(max(val, min_v), max_v)
}

fn apply_line_segment(line: &LineMask, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
    if mask_buf.is_empty() {
        return MaskResult::FullCover;
    }
    line.apply(mask_buf, abs_x, abs_y)
}
