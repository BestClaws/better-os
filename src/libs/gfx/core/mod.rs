/// Core rendering types
/// 
/// This module contains the fundamental types used throughout the rendering system:
/// - Color types (Color, ColorAlpha, Opacity, Hsv, ColorFormat)
/// - Geometric types (Point, PointF, Rect)
/// - Blend operations (BlendMode, BlendDescriptor)

pub mod blend;
pub mod color;
pub mod geometry;

pub use blend::{BlendDescriptor, BlendMode, BlendSource};
pub use color::{Color, ColorAlpha, ColorFormat, Hsv, Opacity};
pub use geometry::{Point, PointF, Rect};
