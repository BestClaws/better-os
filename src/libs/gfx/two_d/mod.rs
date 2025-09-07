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
pub mod draw; // Fluent, builder-based API
pub mod canvas2d;

// Core types and traits
pub use types::{Point, Size, Rect, Rgb565, Rgba8888};
pub use raster::Rasterizer;
pub use draw::{StrokeStyle, FillStyle, CornerRadii as CornerRadiiPx, gradient, gradient_vertical, gradient_angle, stroke, radial, LinearMode, GradientSpec, RadialSpec};

// Optimized primitive drawing functions
// Keep primitives internal to fluent layer; avoid re-exporting low-level draw_* APIs.
// Existing internal modules rely on them via fully qualified paths.

// High-performance gradient rendering
pub use gradients::{
    LinearGradient, RadialGradient, 
    fill_rect_linear_gradient, fill_rect_radial_gradient, fill_rect_rgba
};

// Space-grade text rendering system
pub use text::{TextRenderer, TextOptions};
pub use fonts::{FONT_8X8};
pub use canvas2d::Canvas2D;

