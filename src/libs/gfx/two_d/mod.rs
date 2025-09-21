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
pub mod fluent; // New fluent API - primitives work directly with Canvas2D
// pub mod draw; // OLD - removed
pub mod canvas2d;
pub mod paint; // Generic paint samplers

// Core types and traits
pub use types::{Point, Size, Rect, Rgba8888};
pub use raster::Rasterizer;

// New fluent API - clean, direct, fast
pub use fluent::{Paint, Stroke, Rect as FluentRect, Circle, Line, Arc};

// Optimized primitive drawing functions
// Keep primitives internal to fluent layer; avoid re-exporting low-level draw_* APIs.
// Existing internal modules rely on them via fully qualified paths.

// High-performance gradient rendering
pub use gradients::{
    LinearGradient, RadialGradient,
};

// Space-grade text rendering system
pub use text::{TextRenderer, TextOptions};
pub use fonts::{FONT_8X8};
pub use canvas2d::Canvas2D;

