/// Gradient computation matching LVGL's system
use crate::color::{Rgba8888, lerp_color};
use crate::types::{GradDir, Gradient, Opa};
use crate::math::{atan2_deg, frac_255};

/// Compute color from gradient at a specific position
pub fn gradient_get_color(
    grad: &Gradient,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    cx: i32,
    cy: i32,
) -> (Rgba8888, Opa) {
    if grad.stops_count < 2 {
        return (grad.stops[0].color, grad.stops[0].opa);
    }

    let frac = match grad.dir {
        GradDir::None => 0,
        GradDir::Hor => {
            // Horizontal gradient across width
            if width <= 0 {
                0
            } else {
                frac_255(x, width)
            }
        }
        GradDir::Ver => {
            // Vertical gradient across height
            if height <= 0 {
                0
            } else {
                frac_255(y, height)
            }
        }
        GradDir::Radial => {
            // Radial gradient from center
            let dx = x - cx;
            let dy = y - cy;
            let dist_sq = dx * dx + dy * dy;
            let max_radius = ((width.max(height)) / 2).max(1);
            let max_dist_sq = max_radius * max_radius;
            
            if dist_sq >= max_dist_sq {
                255
            } else {
                frac_255(dist_sq, max_dist_sq)
            }
        }
        GradDir::Conical => {
            // Conical gradient (angle-based)
            let dx = x - cx;
            let dy = y - cy;
            let angle = atan2_deg(dy, dx);
            // Map [0, 360) to [0, 255]
            ((angle * 255) / 360) as u8
        }
    };

    // Interpolate between the two stops
    let color = lerp_color(grad.stops[0].color, grad.stops[1].color, frac);
    let opa = crate::math::lerp_u8(grad.stops[0].opa, grad.stops[1].opa, frac);

    (color, opa)
}

/// Get gradient color for horizontal gradients (optimized)
#[inline]
pub fn gradient_get_color_hor(grad: &Gradient, x: i32, width: i32) -> (Rgba8888, Opa) {
    if width <= 0 {
        return (grad.stops[0].color, grad.stops[0].opa);
    }
    let frac = frac_255(x, width);
    let color = lerp_color(grad.stops[0].color, grad.stops[1].color, frac);
    let opa = crate::math::lerp_u8(grad.stops[0].opa, grad.stops[1].opa, frac);
    (color, opa)
}

/// Get gradient color for vertical gradients (optimized)
#[inline]
pub fn gradient_get_color_ver(grad: &Gradient, y: i32, height: i32) -> (Rgba8888, Opa) {
    if height <= 0 {
        return (grad.stops[0].color, grad.stops[0].opa);
    }
    let frac = frac_255(y, height);
    let color = lerp_color(grad.stops[0].color, grad.stops[1].color, frac);
    let opa = crate::math::lerp_u8(grad.stops[0].opa, grad.stops[1].opa, frac);
    (color, opa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horizontal_gradient() {
        let grad = Gradient::horizontal(Rgba8888::RED, Rgba8888::BLUE);
        
        // At start, should be red
        let (color, _) = gradient_get_color_hor(&grad, 0, 100);
        assert_eq!(color.red(), 255);
        assert_eq!(color.blue(), 0);

        // At end, should be blue
        let (color, _) = gradient_get_color_hor(&grad, 99, 100);
        assert_eq!(color.blue() > 200, true);
    }
}
