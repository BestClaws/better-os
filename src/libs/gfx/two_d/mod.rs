/// High-performance 2D graphics library for space-grade embedded systems.
/// 
/// This module provides optimized 2D graphics primitives and operations designed
/// for maximum performance in embedded systems and real-time applications.
/// 
/// Key features:
/// - Space-grade performance optimizations
/// - Anti-aliased rendering algorithms
/// - Optimized gradient rendering
/// - Efficient pixel operations
/// - Cache-friendly memory layouts
/// - SIMD-friendly operation batching

pub mod types;
pub mod raster;
pub mod primitives;
pub mod gradients;
pub mod text;
pub mod fonts;

// Core types and traits
pub use types::{Point, Size, Rect, Rgb565, Rgba8888};
pub use raster::Rasterizer;

// Optimized primitive drawing functions
pub use primitives::{
    draw_line_aa, draw_line_rgba_aa, draw_arc_aa, draw_arc, 
    draw_rect_outline_aa, fill_rounded_rect, fill_circle, fill_rect
};

// High-performance gradient rendering
pub use gradients::{
    LinearGradient, RadialGradient, 
    fill_rect_linear_gradient, fill_rect_radial_gradient, fill_rect_rgba
};

// Space-grade text rendering system
pub use text::{TextRenderer, TextOptions};
pub use fonts::{FONT_8X8};

