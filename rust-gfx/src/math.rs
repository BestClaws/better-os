/// Math utilities matching LVGL's algorithms
/// These functions must match LVGL exactly for pixel-perfect rendering

/// Fast integer square root (using binary search)
#[inline]
pub fn isqrt(n: u32) -> u32 {
    if n < 2 {
        return n;
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

/// Alias for isqrt matching LVGL's naming
#[inline]
pub fn sqrt32(n: u32) -> u32 {
    isqrt(n)
}

/// Fast atan2 in degrees (matching LVGL's implementation)
/// Returns angle in range [0, 360)
#[inline]
pub fn atan2_deg(y: i32, x: i32) -> i32 {
    if x == 0 && y == 0 {
        return 0;
    }

    let ay = y.abs();
    let ax = x.abs();

    let mut ang = (ay * 45) / (ax + ay + 1);

    if x < 0 {
        ang = 180 - ang;
    }

    if y < 0 {
        ang = 360 - ang;
    }

    ang
}

/// Check if an angle is within a given arc range
/// All angles in degrees [0, 360)
#[inline]
pub fn angle_in_range(angle: i32, start: i32, end: i32) -> bool {
    let angle = angle % 360;
    let start = start % 360;
    let end = end % 360;

    if start <= end {
        angle >= start && angle <= end
    } else {
        // Wraps around 0
        angle >= start || angle <= end
    }
}

/// Clamp a value between min and max
#[inline]
pub fn clamp<T: Ord>(val: T, min: T, max: T) -> T {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

/// Absolute value
#[inline]
pub fn abs(val: i32) -> i32 {
    if val < 0 {
        -val
    } else {
        val
    }
}

/// Calculate distance squared (avoids sqrt)
#[inline]
pub fn dist_sq(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Calculate anti-aliasing coverage based on distance
/// This is LVGL's 1-pixel AA approach using squared distance
#[inline]
pub fn aa_coverage_sq(dist_sq: i32, r: i32) -> u8 {
    let r_sq = r * r;
    let r_next_sq = (r + 1) * (r + 1);

    if dist_sq <= r_sq {
        255
    } else if dist_sq >= r_next_sq {
        0
    } else {
        let t = r_next_sq - dist_sq;
        ((t * 255) / (r_next_sq - r_sq)) as u8
    }
}

/// Linear interpolation (0-255 scale)
#[inline]
pub fn lerp_u8(a: u8, b: u8, frac: u8) -> u8 {
    if frac == 0 {
        return a;
    }
    if frac == 255 {
        return b;
    }
    let inv = 255 - frac;
    ((a as u32 * inv as u32 + b as u32 * frac as u32) / 255) as u8
}

/// Calculate fractional position (returns 0-255)
/// Maps input range 0..(denom-1) to output range 0..255
#[inline]
pub fn frac_255(num: i32, denom: i32) -> u8 {
    if denom == 0 {
        return 0;
    }
    // Simple division without rounding for gradients
    let frac = ((num * 255) / denom).max(0).min(255);
    frac as u8
}

/// Bezier interpolation (for rounded corners, etc.)
#[inline]
pub fn bezier_3(t: i32, p0: i32, p1: i32, p2: i32) -> i32 {
    // t is 0-256
    let t1 = 256 - t;
    ((t1 * t1 * p0) >> 16) + ((2 * t1 * t * p1) >> 16) + ((t * t * p2) >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isqrt() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(100), 10);
    }

    #[test]
    fn test_atan2_deg() {
        // East
        assert_eq!(atan2_deg(0, 1), 0);
        // North
        let north = atan2_deg(1, 0);
        assert!(north >= 85 && north <= 95);
    }

    #[test]
    fn test_angle_in_range() {
        assert!(angle_in_range(45, 0, 90));
        assert!(!angle_in_range(100, 0, 90));
        assert!(angle_in_range(10, 350, 20));  // Wraps around
    }
}
