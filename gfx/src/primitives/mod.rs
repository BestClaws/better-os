//! Graphics primitives

pub mod features;
pub mod font;
pub mod rectangle;

pub use features::fill::FillStyle;
pub use features::gradient::{Gradient, GradientStop};
pub use features::stroke::{StrokeColor, StrokeStyle};
pub use rectangle::{
    CornerRadius, Rectangle, reset_temp_allocation_peak, temp_allocation_peak_bytes,
};
