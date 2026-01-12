/// Masking operations for rounded corners and complex shapes
/// Matches LVGL's mask system
use crate::types::{Area, Opa, OPA_COVER};
use crate::math::{aa_coverage_sq, dist_sq};

/// Radius mask for rounded corners
pub struct RadiusMask {
    area: Area,
    radius: i32,
    outer: bool,
}

impl RadiusMask {
    pub fn new(area: Area, radius: i32, outer: bool) -> Self {
        Self { area, radius, outer }
    }

    /// Get mask value for a point (0 = fully masked, 255 = fully visible)
    pub fn get_mask_value(&self, x: i32, y: i32) -> Opa {
        if self.radius == 0 {
            return OPA_COVER;
        }

        let width = self.area.x2 - self.area.x1 + 1;
        let height = self.area.y2 - self.area.y1 + 1;

        // Determine which corner (if any) this point is in
        let r = self.radius;

        // Top-left corner
        let tl_x = self.area.x1 + r;
        let tl_y = self.area.y1 + r;

        // Top-right corner
        let tr_x = self.area.x2 - r;
        let tr_y = self.area.y1 + r;

        // Bottom-left corner
        let bl_x = self.area.x1 + r;
        let bl_y = self.area.y2 - r;

        // Bottom-right corner
        let br_x = self.area.x2 - r;
        let br_y = self.area.y2 - r;

        // Check which corner region we're in
        let (cx, cy) = if x < tl_x && y < tl_y {
            // Top-left
            (tl_x, tl_y)
        } else if x > tr_x && y < tr_y {
            // Top-right
            (tr_x, tr_y)
        } else if x < bl_x && y > bl_y {
            // Bottom-left
            (bl_x, bl_y)
        } else if x > br_x && y > br_y {
            // Bottom-right
            (br_x, br_y)
        } else {
            // Not in a corner region
            return OPA_COVER;
        };

        // Calculate distance from corner center
        let dist_sq = dist_sq(x, y, cx, cy);
        let coverage = aa_coverage_sq(dist_sq, r);

        if self.outer {
            255 - coverage
        } else {
            coverage
        }
    }

    /// Apply mask to a line buffer
    pub fn apply_to_line(&self, y: i32, x_start: i32, mask_buf: &mut [Opa]) {
        for (i, m) in mask_buf.iter_mut().enumerate() {
            let x = x_start + i as i32;
            let mask_val = self.get_mask_value(x, y);
            // Multiply existing mask value
            *m = ((*m as u32 * mask_val as u32) / 255) as Opa;
        }
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
