#![no_std]

extern crate alloc;

// Public modules
pub mod color;
pub mod fluent;
pub mod masks;
pub mod math;
pub mod primitives;
pub mod rasterizer;
mod rasterizer_rgb565;
pub mod surface;
pub mod three_d;
pub mod types;

// Public API exports
pub use color::{blend_colors, lerp_color, Rgba8888};
pub use rasterizer::Rasterizer;
pub use rasterizer_rgb565::Rgb565Rasterizer;
pub use surface::{ClampedRect, ClampedSpan};
pub use types::*;
