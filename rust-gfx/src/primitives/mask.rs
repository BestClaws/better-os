use crate::math::{aa_coverage_sq, dist_sq};
use crate::primitives::circle_cache::CircleCache;
/// Masking operations for rounded corners and complex shapes
/// Matches LVGL's mask system
use crate::types::{Area, Opa, OPA_COVER};

/// Radius mask for rounded corners (LVGL-compatible)
pub struct RadiusMask {
    area: Area,
    radius: i32,
    outer: bool,
    circle: Option<CircleCache>,
}

impl RadiusMask {
    pub fn new(area: Area, radius: i32, outer: bool) -> Self {
        // Clamp radius to not exceed half the shortest side
        let width = area.x2 - area.x1 + 1;
        let height = area.y2 - area.y1 + 1;
        let short_side = width.min(height);
        let radius = radius.min(short_side / 2).max(0);

        let circle = if radius > 0 {
            Some(CircleCache::new(radius))
        } else {
            None
        };

        Self {
            area,
            radius,
            outer,
            circle,
        }
    }

    /// Get access to the circle cache (for debugging)
    pub fn get_cache(&self) -> Option<&CircleCache> {
        self.circle.as_ref()
    }

    /// Apply mask to a line buffer (LVGL scanline approach)
    /// This matches lv_draw_mask_radius exactly
    pub fn apply_to_line(&self, y: i32, x_start: i32, mask_buf: &mut [Opa]) {
        let len = mask_buf.len() as i32;

        let radius = self.radius;

        // Check if line is outside the rect (and not in corner radius region)
        if y < self.area.y1 || y > self.area.y2 {
            if self.outer {
                // Inverted mask: outside vertical range means no change (full cover)
                return;
            } else {
                // Non-inverted mask outside rect - clear all (transparent)
                for m in mask_buf.iter_mut() {
                    *m = 0;
                }
                return;
            }
        }

        let w = self.area.x2 - self.area.x1 + 1;
        let h = self.area.y2 - self.area.y1 + 1;

        // If in the middle vertical area (no rounding needed)
        if (x_start >= self.area.x1 + radius && x_start + len <= self.area.x2 - radius + 1)
            || (y >= self.area.y1 + radius && y <= self.area.y2 - radius)
        {
            if !self.outer {
                // Clear edges outside rect
                let last = self.area.x1 - x_start;
                if last > 0 && last < len {
                    for i in 0..last {
                        mask_buf[i as usize] = 0;
                    }
                }
                let first = self.area.x2 - x_start + 1;
                if first < len && first > 0 {
                    for i in first..len {
                        mask_buf[i as usize] = 0;
                    }
                }
            } else {
                // Clear middle
                let first = (self.area.x1 - x_start).max(0);
                let last = (self.area.x2 - x_start + 1).min(len);
                if first < last {
                    for i in first..last {
                        mask_buf[i as usize] = 0;
                    }
                }
            }
            return;
        }

        // Get circle data
        let Some(ref circle) = self.circle else {
            return;
        };

        // Convert to relative coordinates (matching LVGL)
        let rel_y = y - self.area.y1;

        // Determine which y in the circle we're at (matching LVGL exactly)
        // Handle negative rel_y for lines above the rect
        let cir_y = if rel_y < 0 {
            // Above rect - mirror the calculation
            radius + rel_y
        } else if rel_y < radius {
            radius - rel_y - 1
        } else {
            rel_y - (h - radius)
        };

        let Some((aa_opa, x_offset)) = circle.get_line(cir_y) else {
            return;
        };

        let aa_len = aa_opa.len() as i32;
        let k = self.area.x1 - x_start;
        let cir_x_right = k + w - radius + x_offset;
        let cir_x_left = k + radius - x_offset - 1;

        if !self.outer {
            // Apply AA to corners
            for i in 0..aa_len {
                let opa = aa_opa[(aa_len - i - 1) as usize];

                let right_idx = cir_x_right + i;
                if right_idx >= 0 && right_idx < len {
                    mask_buf[right_idx as usize] =
                        Self::mask_mix(opa, mask_buf[right_idx as usize]);
                }

                let left_idx = cir_x_left - i;
                if left_idx >= 0 && left_idx < len {
                    mask_buf[left_idx as usize] = Self::mask_mix(opa, mask_buf[left_idx as usize]);
                }
            }

            // Clear outside areas
            let right_clear = (cir_x_right + aa_len).max(0).min(len);
            for i in right_clear..len {
                mask_buf[i as usize] = 0;
            }

            let left_clear = (cir_x_left - aa_len + 1).max(0).min(len);
            for i in 0..left_clear {
                mask_buf[i as usize] = 0;
            }
        } else {
            // Outer mask (inverted)
            for i in 0..aa_len {
                let opa = 255 - aa_opa[(aa_len - 1 - i) as usize];

                let right_idx = cir_x_right + i;
                if right_idx >= 0 && right_idx < len {
                    mask_buf[right_idx as usize] =
                        Self::mask_mix(opa, mask_buf[right_idx as usize]);
                }

                let left_idx = cir_x_left - i;
                if left_idx >= 0 && left_idx < len {
                    mask_buf[left_idx as usize] = Self::mask_mix(opa, mask_buf[left_idx as usize]);
                }
            }

            // Clear middle
            let clr_start = (cir_x_left + 1).max(0).min(len);
            let clr_end = cir_x_right.max(0).min(len);
            for i in clr_start..clr_end {
                mask_buf[i as usize] = 0;
            }
        }
    }

    #[inline]
    fn mask_mix(mask_act: Opa, mask_new: Opa) -> Opa {
        if mask_new >= 255 {
            return mask_act;
        }
        if mask_new <= 0 {
            return 0;
        }
        let product = mask_act as u32 * mask_new as u32;
        ((product * 0x8081) >> 23) as Opa
    }

    /// Legacy per-pixel mask (slower, but compatible)
    /// For scanline rendering, use apply_to_line instead
    #[allow(dead_code)]
    pub fn get_mask_value(&self, x: i32, y: i32) -> Opa {
        // Fallback: use apply_to_line for single pixel
        let mut buf = [255u8];
        self.apply_to_line(y, x, &mut buf);
        buf[0]
    }
}

/// Line mask (for drawing lines with anti-aliasing)
pub struct LineMask {
    pub p1_x: i32,
    pub p1_y: i32,
    pub p2_x: i32,
    pub p2_y: i32,
    pub width: i32,
}

impl LineMask {
    pub fn new(p1_x: i32, p1_y: i32, p2_x: i32, p2_y: i32, width: i32) -> Self {
        Self {
            p1_x,
            p1_y,
            p2_x,
            p2_y,
            width,
        }
    }

    /// Get distance from point to line
    pub fn point_to_line_dist_sq(&self, x: i32, y: i32) -> i32 {
        let dx = self.p2_x - self.p1_x;
        let dy = self.p2_y - self.p1_y;
        let len_sq = dx * dx + dy * dy;

        if len_sq == 0 {
            // Point line, return distance to p1
            return dist_sq(x, y, self.p1_x, self.p1_y);
        }

        // Calculate parametric position along line
        let px = x - self.p1_x;
        let py = y - self.p1_y;
        let t = ((px * dx + py * dy) * 256) / len_sq;

        if t < 0 {
            // Before line start
            dist_sq(x, y, self.p1_x, self.p1_y)
        } else if t > 256 {
            // After line end
            dist_sq(x, y, self.p2_x, self.p2_y)
        } else {
            // Perpendicular distance to line
            let proj_x = self.p1_x + (dx * t) / 256;
            let proj_y = self.p1_y + (dy * t) / 256;
            dist_sq(x, y, proj_x, proj_y)
        }
    }

    pub fn get_mask_value(&self, x: i32, y: i32) -> Opa {
        let d_sq = self.point_to_line_dist_sq(x, y);
        let half_width = self.width / 2;
        aa_coverage_sq(d_sq, half_width)
    }
}

/// Triangle mask
pub struct TriangleMask {
    pub p1_x: i32,
    pub p1_y: i32,
    pub p2_x: i32,
    pub p2_y: i32,
    pub p3_x: i32,
    pub p3_y: i32,
}

impl TriangleMask {
    pub fn new(p1_x: i32, p1_y: i32, p2_x: i32, p2_y: i32, p3_x: i32, p3_y: i32) -> Self {
        Self {
            p1_x,
            p1_y,
            p2_x,
            p2_y,
            p3_x,
            p3_y,
        }
    }

    /// Check if point is inside triangle (using barycentric coordinates)
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        let sign = |p1x: i32, p1y: i32, p2x: i32, p2y: i32, p3x: i32, p3y: i32| -> i32 {
            (p1x - p3x) * (p2y - p3y) - (p2x - p3x) * (p1y - p3y)
        };

        let d1 = sign(x, y, self.p1_x, self.p1_y, self.p2_x, self.p2_y);
        let d2 = sign(x, y, self.p2_x, self.p2_y, self.p3_x, self.p3_y);
        let d3 = sign(x, y, self.p3_x, self.p3_y, self.p1_x, self.p1_y);

        let has_neg = (d1 < 0) || (d2 < 0) || (d3 < 0);
        let has_pos = (d1 > 0) || (d2 > 0) || (d3 > 0);

        !(has_neg && has_pos)
    }

    pub fn get_mask_value(&self, x: i32, y: i32) -> Opa {
        if self.contains_point(x, y) {
            OPA_COVER
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radius_mask() {
        let area = Area::new(10, 10, 50, 50);
        let mask = RadiusMask::new(area, 5, false);

        // Center should be fully visible
        assert_eq!(mask.get_mask_value(30, 30), OPA_COVER);

        // Top-left corner should have some masking
        let val = mask.get_mask_value(10, 10);
        assert!(val < OPA_COVER);
    }

    #[test]
    fn test_triangle_contains() {
        let mask = TriangleMask::new(0, 0, 10, 0, 5, 10);
        assert!(mask.contains_point(5, 5));
        assert!(!mask.contains_point(20, 20));
    }
}
