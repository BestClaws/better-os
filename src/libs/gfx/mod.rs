#![allow(dead_code)]

// Re-export core graphics from rust-gfx library
pub use rust_gfx::{
    Arc, Circle, Fill, Line, Rasterizer, Rgb565Rasterizer, Rgba8888, RoundedRect,
    Shape,
};

// Re-export 3D rendering
pub use rust_gfx::{draw_model, parse_binary_stl, Model, Quaternion, RenderOptions, StlError, Vec3};

pub const BLACK: u16 = 0x0000;

/// Convert RGBA8888 to 4-bit grayscale + separate alpha
#[inline(always)]
pub fn rgba8888_to_gray4_and_alpha(color: u32) -> (u8, u8) {
    let r = ((color >> 24) & 0xFF) as u8;
    let g = ((color >> 16) & 0xFF) as u8;
    let b = ((color >> 8) & 0xFF) as u8;
    let a = (color & 0xFF) as u8;
    // ITU-R BT.601 luma coefficients
    let gray8 = ((r as u16 * 77 + g as u16 * 150 + b as u16 * 29) >> 8) as u8;
    let gray4 = gray8 >> 4; // Convert 8-bit to 4-bit
    (gray4, a)
}