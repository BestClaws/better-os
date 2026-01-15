#![no_std]

extern crate alloc;

// Public modules
pub mod color;
pub mod masks;
pub mod math;
pub mod primitives;
pub mod rasterizer;
pub mod three_d;
pub mod types;

#[cfg(feature = "std")]
pub mod bmp;
#[cfg(feature = "std")]
pub mod canvas;

// Public API exports
#[cfg(feature = "std")]
pub use canvas::Canvas;
pub use color::{blend_colors, lerp_color, Rgba8888};
pub use rasterizer::{Rasterizer, Rgb565Rasterizer};
pub use types::*;
