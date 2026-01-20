use alloc::vec;
use alloc::vec::Vec;
use core::cmp::min;

use crate::types::{Area, Opa};
use super::{clamp_i32, mask_mix, MaskResult};

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
