#![no_std]

extern crate alloc;

// Public modules
pub mod color;
pub mod types;
pub mod math;
pub mod primitives;
pub mod rasterizer;
pub mod masks;

#[cfg(feature = "std")]
pub mod canvas;
#[cfg(feature = "std")]
pub mod bmp;

// Public API exports
pub use color::{Rgba8888, blend_colors, lerp_color};
pub use types::*;
pub use rasterizer::{Rasterizer, Rgb565Rasterizer};
#[cfg(feature = "std")]
pub use canvas::Canvas;
