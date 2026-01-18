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

// Public API exports
pub use color::{blend_colors, lerp_color, Rgba8888};
pub use rasterizer::{Rasterizer};
pub use types::*;
