//! Graphics primitives

pub mod font;
pub mod features;
pub mod rectangle;

pub use rectangle::{CornerRadius, Rectangle};
pub use features::fill::FillStyle;
pub use features::gradient::{Gradient, GradientStop};
pub use features::stroke::{StrokeColor, StrokeStyle};
