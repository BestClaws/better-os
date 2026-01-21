/// Gradient computation matching LVGL's system
use crate::color::{lerp_color, Rgba8888};
use crate::math::{atan2_deg, frac_255};
use crate::types::{GradDir, Gradient, Opa};

use micromath::F32Ext;

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
    let (stops, stop_len) = normalize_stops(grad);
    if stop_len == 0 {
        return (Rgba8888::BLACK, 0);
    }
    if stop_len == 1 {
        let stop = stops[0];
        return (stop.color, stop.opa);
    }

    let frac = match grad.dir {
        GradDir::None => 0u8,
        GradDir::Hor => {
            // Horizontal gradient: map pixels 0..(width-1) to 0..255
            // Last pixel (width-1) should map to 255
            if width <= 1 {
                255u8
            } else {
                frac_255(x, width - 1)
            }
        }
        GradDir::Ver => {
            // Vertical gradient: map pixels 0..(height-1) to 0..255
            if height <= 1 {
                255u8
            } else {
                frac_255(y, height - 1)
            }
        }
        GradDir::Radial => {
            // In LVGL simple mode (LV_USE_DRAW_SW_COMPLEX_GRADIENTS=0), radial gradients
            // fall back to horizontal gradients. Match this behavior.
            if width <= 1 {
                255u8
            } else {
                frac_255(x, width - 1)
            }
        }
        GradDir::Conical => {
            // In LVGL simple mode (LV_USE_DRAW_SW_COMPLEX_GRADIENTS=0), conical gradients
            // fall back to horizontal gradients. Match this behavior.
            if width <= 1 {
                255u8
            } else {
                frac_255(x, width - 1)
            }
        }
    };

    sample_stops(stops, stop_len, i32::from(frac))
}

/// Get gradient color for horizontal gradients (optimized)
#[inline]
pub fn gradient_get_color_hor(grad: &Gradient, x: i32, width: i32) -> (Rgba8888, Opa) {
    if width <= 0 {
        let (stops, len) = normalize_stops(grad);
        if len == 0 {
            return (Rgba8888::BLACK, 0);
        }
        return (stops[0].color, stops[0].opa);
    }
    let frac = frac_255(x, width);
    let (stops, len) = normalize_stops(grad);
    sample_stops(stops, len, i32::from(frac))
}

/// Get gradient color for vertical gradients (optimized)
#[inline]
pub fn gradient_get_color_ver(grad: &Gradient, y: i32, height: i32) -> (Rgba8888, Opa) {
    if height <= 0 {
        let (stops, len) = normalize_stops(grad);
        if len == 0 {
            return (Rgba8888::BLACK, 0);
        }
        return (stops[0].color, stops[0].opa);
    }
    let frac = frac_255(y, height);
    let (stops, len) = normalize_stops(grad);
    sample_stops(stops, len, i32::from(frac))
}

fn normalize_stops(grad: &Gradient) -> (&[crate::types::GradStop], usize) {
    let count = grad
        .stops_count
        .min(crate::types::MAX_GRADIENT_STOPS)
        .max(1);
    (&grad.stops[..count], count)
}

fn sample_stops(stops: &[crate::types::GradStop], len: usize, frac: i32) -> (Rgba8888, Opa) {
    if len == 0 {
        return (Rgba8888::BLACK, 0);
    }
    if len == 1 {
        let stop = stops[0];
        return (stop.color, stop.opa);
    }

    let mut prev = stops[0];
    for stop in stops.iter().copied().skip(1) {
        if frac <= stop.frac as i32 {
            let span = (stop.frac as i32 - prev.frac as i32).max(1);
            let rel = ((frac - prev.frac as i32) as f32 / span as f32).clamp(0.0, 1.0);
            let blend = (rel * 255.0).round().clamp(0.0, 255.0) as u8;
            let color = lerp_color(prev.color, stop.color, blend);
            let opa = crate::math::lerp_u8(prev.opa, stop.opa, blend);
            return (color, opa);
        }
        prev = stop;
    }
    (prev.color, prev.opa)
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
