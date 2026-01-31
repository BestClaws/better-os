#![no_std]

extern crate alloc;

// Public modules
pub mod colors;
pub mod masks;
pub mod math;
pub mod primitives;
pub mod rasterizer;
pub mod three_d;
pub mod types;

// Public API exports
pub use colors::{blend_colors, lerp_color, Color};
pub use rasterizer::{Rasterizer};
pub use types::*;
