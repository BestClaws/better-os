/// Math utilities matching LVGL's algorithms
/// These functions must match LVGL exactly for pixel-perfect rendering

/// Sine lookup table for 0..=90 degrees (LVGL compat).
/// Copied from LVGL (MIT License) to ensure identical trig behavior.
const SIN0_90_TABLE: [i32; 91] = [
    0, 572, 1144, 1715, 2286, 2856, 3425, 3993, 4560, 5126, 5690, 6252, 6813, 7371, 7927, 8481,
    9032, 9580, 10126, 10668, 11207, 11743, 12275, 12803, 13328, 13848, 14365, 14876, 15384, 15886,
    16384, 16877, 17364, 17847, 18324, 18795, 19261, 19720, 20174, 20622, 21063, 21498, 21926,
    22348, 22763, 23170, 23571, 23965, 24351, 24730, 25102, 25466, 25822, 26170, 26510, 26842,
    27166, 27482, 27789, 28088, 28378, 28660, 28932, 29197, 29452, 29698, 29935, 30163, 30382,
    30592, 30792, 30983, 31164, 31336, 31499, 31651, 31795, 31928, 32052, 32166, 32270, 32365,
    32449, 32524, 32588, 32643, 32688, 32723, 32748, 32763, 32768,
];

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

/// Fixed-point sine matching lv_trigo_sin (returns value scaled by 32768).
#[inline]
pub fn trigo_sin(mut angle: i32) -> i32 {
    while angle < 0 {
        angle += 360;
    }
    while angle >= 360 {
        angle -= 360;
    }

    let value = if angle < 90 {
        SIN0_90_TABLE[angle as usize]
    } else if angle < 180 {
        SIN0_90_TABLE[(180 - angle) as usize]
    } else if angle < 270 {
        -SIN0_90_TABLE[(angle - 180) as usize]
    } else {
        -SIN0_90_TABLE[(360 - angle) as usize]
    };

    match value {
        32767 => 32768,
        -32767 => -32768,
        _ => value,
    }
}

/// Fixed-point cosine matching lv_trigo_cos.
#[inline]
pub fn trigo_cos(angle: i32) -> i32 {
    trigo_sin(angle + 90)
}

/// Alias for isqrt matching LVGL's naming
#[inline]
pub fn sqrt32(n: u32) -> u32 {
    isqrt(n)
}

/// Fast atan2 in degrees (matching LVGL's lv_atan2)
/// Takes (x, y) parameter order like LVGL, not standard atan2(y, x)
/// Returns angle in range [0, 360) where 0° is up, 90° is right
#[inline]
pub fn atan2_deg(x: i32, y: i32) -> i32 {
    if x == 0 && y == 0 {
        return 0;
    }

    let mut negflag = 0u8;
    let mut ux = x;
    let mut uy = y;

    // Save sign flags and make values positive
    if x < 0 {
        negflag |= 0x01;
        ux = -x;
    }
    if y < 0 {
        negflag |= 0x02;
        uy = -y;
    }

    // Calculate scaled degrees (0-45 range)
    let mut degree = if ux > uy {
        negflag |= 0x10;
        (uy * 45) / ux
    } else {
        (ux * 45) / uy
    };

    // Compensate for error curve (LVGL's compensation table)
    let mut comp = 0;
    if degree > 22 {
        if degree <= 44 {
            comp += 1;
        }
        if degree <= 41 {
            comp += 1;
        }
        if degree <= 37 {
            comp += 1;
        }
        if degree <= 32 {
            comp += 1;
        }
    } else {
        if degree >= 2 {
            comp += 1;
        }
        if degree >= 6 {
            comp += 1;
        }
        if degree >= 10 {
            comp += 1;
        }
        if degree >= 15 {
            comp += 1;
        }
    }
    degree += comp;

    // Invert if X>Y octant (makes 0-45 into 90-45)
    if negflag & 0x10 != 0 {
        degree = 90 - degree;
    }

    // Map to correct quadrant based on sign flags
    if negflag & 0x02 != 0 {
        // -Y
        if negflag & 0x01 != 0 {
            // -Y -X (quadrant 3)
            degree = 180 + degree;
        } else {
            // -Y +X (quadrant 4)
            degree = 180 - degree;
        }
    } else {
        // +Y
        if negflag & 0x01 != 0 {
            // +Y -X (quadrant 2)
            degree = 360 - degree;
        }
        // else +Y +X (quadrant 1): degree unchanged
    }

    degree
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
        assert!(angle_in_range(10, 350, 20)); // Wraps around
    }
}
