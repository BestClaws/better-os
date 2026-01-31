/// Gradient computation matching LVGL's system
use crate::colors::{lerp_color, Color};
use crate::math::{atan2_deg, frac_255};
use crate::types::{GradDir, Gradient, Opacity};

/// Compute color from gradient at a specific position
pub fn gradient_get_color(
    grad: &Gradient,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    cx: i32,
    cy: i32,
) -> (Color, Opacity) {
    if grad.stops_count < 2 {
        return (grad.stops[0].color, grad.stops[0].opa);
    }

    let frac = match grad.dir {
        GradDir::None => 0,
        GradDir::Hor => {
            // Horizontal gradient: map pixels 0..(width-1) to 0..255
            // Last pixel (width-1) should map to 255
            if width <= 1 {
                255
            } else {
                frac_255(x, width - 1)
            }
        }
        GradDir::Ver => {
            // Vertical gradient: map pixels 0..(height-1) to 0..255
            if height <= 1 {
                255
            } else {
                frac_255(y, height - 1)
            }
        }
        GradDir::Radial => {
            // In LVGL simple mode (LV_USE_DRAW_SW_COMPLEX_GRADIENTS=0), radial gradients
            // fall back to horizontal gradients. Match this behavior.
            if width <= 1 {
                255
            } else {
                frac_255(x, width - 1)
            }
        }
        GradDir::Conical => {
            // In LVGL simple mode (LV_USE_DRAW_SW_COMPLEX_GRADIENTS=0), conical gradients
            // fall back to horizontal gradients. Match this behavior.
            if width <= 1 {
                255
            } else {
                frac_255(x, width - 1)
            }
        }
    };

    // Interpolate between the two stops
    let color = lerp_color(grad.stops[0].color, grad.stops[1].color, frac);
    let opa = crate::math::lerp_u8(grad.stops[0].opa, grad.stops[1].opa, frac);

    (color, opa)
}

/// Get gradient color for horizontal gradients (optimized)
#[inline]
pub fn gradient_get_color_hor(grad: &Gradient, x: i32, width: i32) -> (Color, Opacity) {
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
pub fn gradient_get_color_ver(grad: &Gradient, y: i32, height: i32) -> (Color, Opacity) {
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
        let grad = Gradient::horizontal(Color::RED, Color::BLUE);

        // At start, should be red
        let (color, _) = gradient_get_color_hor(&grad, 0, 100);
        assert_eq!(color.red(), 255);
        assert_eq!(color.blue(), 0);

        // At end, should be blue
        let (color, _) = gradient_get_color_hor(&grad, 99, 100);
        assert_eq!(color.blue() > 200, true);
    }
}
