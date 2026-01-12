/// Primitive drawing routines matching LVGL exactly
/// Each primitive is in its own file to keep code organized

pub mod rectangle;
pub mod triangle;
pub mod line;
pub mod arc;
pub mod label;
pub mod blur;
pub mod gradient;
pub mod mask;

// Re-exports for convenience
pub use rectangle::*;
pub use triangle::*;
pub use line::*;
pub use arc::*;
pub use label::*;
pub use blur::*;
pub use gradient::*;
pub use mask::*;
