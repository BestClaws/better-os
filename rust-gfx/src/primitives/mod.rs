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
#[cfg(feature = "std")]
pub use triangle::*;
#[cfg(feature = "std")]
pub use line::*;
#[cfg(feature = "std")]
pub use arc::*;
#[cfg(feature = "std")]
pub use label::*;
#[cfg(feature = "std")]
pub use blur::*;
pub use gradient::*;
pub use mask::*;
