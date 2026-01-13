pub mod arc;
pub mod blur;
pub mod circle_cache;
pub mod gradient;
pub mod label;
pub mod line;
pub mod mask;
/// Primitive drawing routines matching LVGL exactly
/// Each primitive is in its own file to keep code organized
pub mod rectangle;
pub mod triangle;

// Re-exports for convenience
#[cfg(feature = "std")]
pub use arc::*;
#[cfg(feature = "std")]
pub use blur::*;
pub use gradient::*;
#[cfg(feature = "std")]
pub use label::*;
#[cfg(feature = "std")]
pub use line::*;
pub use mask::*;
pub use rectangle::*;
#[cfg(feature = "std")]
pub use triangle::*;
