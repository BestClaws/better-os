pub mod types;
pub mod raster;
pub mod primitives;
pub mod gradients;

pub use types::{Point, Size, Rect, Rgb565, Rgba8888};
pub use raster::Rasterizer;
pub use primitives::{draw_line_aa, draw_line_rgba_aa, draw_arc_aa, draw_arc, draw_rect_outline_aa, fill_rounded_rect, fill_circle};
pub use gradients::{LinearGradient, RadialGradient, fill_rect_linear_gradient, fill_rect_radial_gradient};
pub use primitives::fill_rect;
pub use gradients::fill_rect_rgba;

