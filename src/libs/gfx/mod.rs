#![allow(dead_code)]

// file: src/lib.rs

use crate::system::hal::display::AsyncDisplay;

pub(crate) mod color;
pub mod fill;
mod font;
pub mod rasterizer;
pub mod shapes;

pub use fill::Fill;
pub use rasterizer::Rasterizer;
pub use rasterizer::Rgb565Rasterizer;
pub use shapes::{Arc, Circle, RoundedRect, Shape};

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

#[inline(always)]
pub fn linear_gradient_h(
    start_color: u16,
    end_color: u16,
    start_alpha: u8,
    end_alpha: u8,
    x: i32,
    cx: i32,
    r: i32,
) -> (u16, u8) {
    let frac = (((x - (cx - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    (
        lerp_rgb565(start_color, end_color, frac),
        lerp_u8(start_alpha, end_alpha, frac),
    )
}

#[inline(always)]
pub fn linear_gradient_v(
    start_color: u16,
    end_color: u16,
    start_alpha: u8,
    end_alpha: u8,
    y: i32,
    cy: i32,
    r: i32,
) -> (u16, u8) {
    let frac = (((y - (cy - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    (
        lerp_rgb565(start_color, end_color, frac),
        lerp_u8(start_alpha, end_alpha, frac),
    )
}

// RGBA helpers for pixel-format independent primitives
#[inline(always)]
pub fn lerp_rgba(a: color::Rgba8888, b: color::Rgba8888, frac: u8) -> color::Rgba8888 {
    let inv = 255 - frac;
    let au = a.to_u32();
    let bu = b.to_u32();
    let ar = ((au >> 24) & 0xFF) as u8;
    let ag = ((au >> 16) & 0xFF) as u8;
    let ab = ((au >> 8) & 0xFF) as u8;
    let aa = (au & 0xFF) as u8;

    let br = ((bu >> 24) & 0xFF) as u8;
    let bg = ((bu >> 16) & 0xFF) as u8;
    let bb = ((bu >> 8) & 0xFF) as u8;
    let ba = (bu & 0xFF) as u8;

    let r = (((ar as u16 * inv as u16) + (br as u16 * frac as u16)) / 255) as u8;
    let g = (((ag as u16 * inv as u16) + (bg as u16 * frac as u16)) / 255) as u8;
    let b = (((ab as u16 * inv as u16) + (bb as u16 * frac as u16)) / 255) as u8;
    let a = (((aa as u16 * inv as u16) + (ba as u16 * frac as u16)) / 255) as u8;
    color::Rgba8888::rgba(r, g, b, a)
}

#[inline(always)]
pub fn radial_gradient_rgba_sq(
    inner: color::Rgba8888,
    outer: color::Rgba8888,
    dist2: i32,
    r2: i32,
) -> color::Rgba8888 {
    let frac = ((dist2 * 255) / r2).clamp(0, 255) as u8;
    lerp_rgba(inner, outer, frac)
}

#[inline(always)]
pub fn linear_gradient_h_rgba(
    start: color::Rgba8888,
    end: color::Rgba8888,
    x: i32,
    cx: i32,
    r: i32,
) -> color::Rgba8888 {
    let frac = (((x - (cx - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    lerp_rgba(start, end, frac)
}

#[inline(always)]
pub fn linear_gradient_v_rgba(
    start: color::Rgba8888,
    end: color::Rgba8888,
    y: i32,
    cy: i32,
    r: i32,
) -> color::Rgba8888 {
    let frac = (((y - (cy - r)) * 255) / (r * 2)).clamp(0, 255) as u8;
    lerp_rgba(start, end, frac)
}

#[inline(always)]
pub(crate) fn rgba8888_to_rgb565_and_alpha(color: u32) -> (u16, u8) {
    let r = ((color >> 24) & 0xFF) >> 3;
    let g = ((color >> 16) & 0xFF) >> 2;
    let b = ((color >> 8) & 0xFF) >> 3;
    let a = (color & 0xFF) as u8;
    (((r << 11) | (g << 5) | b) as u16, a)
}

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
