/// High-performance 2D graphics API with anti-aliasing and gradient support
/// 
/// This module provides a comprehensive 2D graphics API optimized for embedded systems
/// with support for:
/// - Primitives: rectangles, circles, lines, arcs, bezier curves
/// - Advanced rendering: gradients (linear, radial, conic), anti-aliasing
/// - Performance: fixed-point math, SIMD-friendly operations, cache optimization
/// - Fluent API: chainable method calls for intuitive usage

pub mod types;
pub mod paint;
pub mod stroke;
pub mod primitives;
pub mod raster;
pub mod canvas2d;

// Re-export core types for convenience
pub use types::{Point, Size, Rgba8888, CornerRadii, AntiAliasing, BlendMode};
pub use fixed::{FixedI32, types::extra::U16};
pub use types::Rect; // Use geometry rect as the main Rect
pub use paint::*;
pub use stroke::*;
pub use primitives::{Rect as PrimitiveRect, Circle, Line, Arc, Bezier, Drawable}; // Primitive rect
pub use raster::Rasterizer;
pub use canvas2d::Canvas2D;
