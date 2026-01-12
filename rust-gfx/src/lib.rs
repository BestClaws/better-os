#![no_std]

// New LVGL-compatible modules
pub mod color_argb;
pub mod types;
#[cfg(feature = "std")]
pub mod canvas;
pub mod math;
pub mod primitives;

#[cfg(feature = "std")]
pub mod bmp;

// Legacy modules (kept for backward compatibility)
pub mod color;
pub mod fill;
pub mod rasterizer;
pub mod shapes;
pub mod three_d;

// New exports
pub use color_argb::{Argb8888, blend_colors, lerp_color};
pub use types::*;
#[cfg(feature = "std")]
pub use canvas::Canvas;

// Legacy exports (deprecated)
#[allow(deprecated)]
pub use color::Rgba8888;
pub use fill::Fill;
pub use rasterizer::Rasterizer;
pub use rasterizer::Rgb565Rasterizer;
pub use shapes::{Arc, Circle, Line, RoundedRect, Shape};
pub use three_d::{draw_model, parse_binary_stl, Model, Quaternion, RenderOptions, StlError, Vec3};

pub const BLACK: u16 = 0x0000;

/// 1px LVGL-style AA coverage using squared distance
#[inline(always)]
pub fn aa_coverage(dist2: i32, r: i32) -> u8 {
    let r2 = r * r;
    let r2_next = (r + 1) * (r + 1);

    if dist2 <= r2 {
        255
    } else if dist2 >= r2_next {
        0
    } else {
        let t = r2_next - dist2;
        ((t * 255) / (r2_next - r2)) as u8
    }
}

/// RGB565 alpha blend
#[inline(always)]
pub fn blend_rgb565(bg: u16, fg: u16, opa: u8) -> u16 {
    if opa == 255 {
        return fg;
    }
    if opa == 0 {
        return bg;
    }

    let inv = 255 - opa;

    let br = ((bg >> 11) & 0x1F) * inv as u16;
    let bg_g = ((bg >> 5) & 0x3F) * inv as u16;
    let bb = (bg & 0x1F) * inv as u16;

    let fr = ((fg >> 11) & 0x1F) * opa as u16;
    let fg_g = ((fg >> 5) & 0x3F) * opa as u16;
    let fb = (fg & 0x1F) * opa as u16;

    (((br + fr) / 255) << 11) | (((bg_g + fg_g) / 255) << 5) | ((bb + fb) / 255)
}

/// RGB565 linear interpolation
#[inline(always)]
pub fn lerp_rgb565(a: u16, b: u16, frac: u8) -> u16 {
    let inv = 255 - frac;

    let r = (((a >> 11) & 0x1F) * inv as u16 + ((b >> 11) & 0x1F) * frac as u16) / 255;

    let g = (((a >> 5) & 0x3F) * inv as u16 + ((b >> 5) & 0x3F) * frac as u16) / 255;

    let b = ((a & 0x1F) * inv as u16 + (b & 0x1F) * frac as u16) / 255;

    (r << 11) | (g << 5) | b
}

/// u8 linear interpolation
#[inline(always)]
pub fn lerp_u8(a: u8, b: u8, frac: u8) -> u8 {
    let inv = 255 - frac;
    ((a as u16 * inv as u16 + b as u16 * frac as u16) / 255) as u8
}

/// Radial gradient (squared distance, no sqrt)
#[inline(always)]
pub fn radial_gradient_sq(
    inner_color: u16,
    outer_color: u16,
    inner_alpha: u8,
    outer_alpha: u8,
    dist2: i32,
    r2: i32,
) -> (u16, u8) {
    let frac = ((dist2 * 255) / r2).clamp(0, 255) as u8;
    (
        lerp_rgb565(inner_color, outer_color, frac),
        lerp_u8(inner_alpha, outer_alpha, frac),
    )
}

/// Convert RGBA8888 to RGB565 + separate alpha
#[inline(always)]
pub fn rgba8888_to_rgb565_and_alpha(rgba: u32) -> (u16, u8) {
    let r = ((rgba >> 24) & 0xFF) as u8;
    let g = ((rgba >> 16) & 0xFF) as u8;
    let b = ((rgba >> 8) & 0xFF) as u8;
    let a = (rgba & 0xFF) as u8;

    let r5 = (r as u16 >> 3) & 0x1F;
    let g6 = (g as u16 >> 2) & 0x3F;
    let b5 = (b as u16 >> 3) & 0x1F;

    let rgb565 = (r5 << 11) | (g6 << 5) | b5;
    (rgb565, a)
}

/// RGBA linear interpolation
#[inline(always)]
pub fn lerp_rgba(a: Rgba8888, b: Rgba8888, frac: u8) -> Rgba8888 {
    let a = a.to_u32();
    let b = b.to_u32();
    let inv = 255 - frac;

    let ar = ((a >> 24) & 0xFF) as u16;
    let ag = ((a >> 16) & 0xFF) as u16;
    let ab = ((a >> 8) & 0xFF) as u16;
    let aa = (a & 0xFF) as u16;

    let br = ((b >> 24) & 0xFF) as u16;
    let bg = ((b >> 16) & 0xFF) as u16;
    let bb = ((b >> 8) & 0xFF) as u16;
    let ba = (b & 0xFF) as u16;

    let r = ((ar * inv as u16 + br * frac as u16) / 255) as u32;
    let g = ((ag * inv as u16 + bg * frac as u16) / 255) as u32;
    let b = ((ab * inv as u16 + bb * frac as u16) / 255) as u32;
    let a = ((aa * inv as u16 + ba * frac as u16) / 255) as u32;

    Rgba8888::from_u32((r << 24) | (g << 16) | (b << 8) | a)
}

#[inline(always)]
pub fn radial_gradient_rgba_sq(
    inner: Rgba8888,
    outer: Rgba8888,
    dist2: i32,
    r2: i32,
) -> Rgba8888 {
    let frac = ((dist2 * 255) / r2).clamp(0, 255) as u8;
    lerp_rgba(inner, outer, frac)
}

#[inline(always)]
pub fn linear_gradient_h_rgba(
    start: Rgba8888,
    end: Rgba8888,
    x: i32,
    cx: i32,
    r: i32,
) -> Rgba8888 {
    let frac = (((x - (cx - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    lerp_rgba(start, end, frac)
}

#[inline(always)]
pub fn linear_gradient_v_rgba(
    start: Rgba8888,
    end: Rgba8888,
    y: i32,
    cy: i32,
    r: i32,
) -> Rgba8888 {
    let frac = (((y - (cy - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    lerp_rgba(start, end, frac)
}

/// Compute the fractional part of "x / N" * 256, as an integer in 0..=255.
#[inline(always)]
pub fn frac256(x: i32, n: i32) -> u8 {
    if n == 0 {
        return 0;
    }
    let rem = x % n;
    ((rem * 256) / n) as u8
}

/// Helper to check if an angle is within a given arc.
#[inline(always)]
pub fn angle_in_range(dx: i32, dy: i32, start: i32, end: i32) -> bool {
    let ang = fast_atan2_deg(dy, dx);

    if start <= end {
        ang >= start && ang <= end
    } else {
        ang >= start || ang <= end
    }
}

#[inline(always)]
pub fn fast_atan2_deg(y: i32, x: i32) -> i32 {
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
