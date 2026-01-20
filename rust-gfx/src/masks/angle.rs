use crate::types::{Opa, Point};
use super::{LineMask, LineSide, MaskResult};

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

            let end_angle_first = ((rel_y as i64 * self.end_line.xy_steep() as i64) >> 10) as i32;
            let mut start_angle_last =
                (((rel_y + 1) as i64 * self.start_line.xy_steep() as i64) >> 10) as i32;

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
        } else if self.start_angle > 180 && self.end_angle > 180 && self.start_angle > self.end_angle {
            if abs_y > self.vertex.y {
                return MaskResult::FullCover;
            }

            let end_angle_first = ((rel_y as i64 * self.end_line.xy_steep() as i64) >> 10) as i32;
            let mut start_angle_last =
                (((rel_y + 1) as i64 * self.start_line.xy_steep() as i64) >> 10) as i32;

            Self::adjust_cross(&mut start_angle_last, self.start_angle);
            Self::adjust_cross(&mut start_angle_last, self.end_angle);

            let dist = (end_angle_first - start_angle_last) >> 1;
            let mut tmp = start_angle_last + dist - rel_x;
            if tmp > len {
                tmp = len;
            }

            let mut res1 = MaskResult::FullCover;
            if tmp > 0 {
                res1 = apply_line_segment(&self.end_line, &mut mask_buf[..tmp as usize], abs_x, abs_y);
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

fn apply_line_segment(line: &LineMask, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
    if mask_buf.is_empty() {
        return MaskResult::FullCover;
    }
    line.apply(mask_buf, abs_x, abs_y)
}
