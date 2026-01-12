/// Primitive drawing routines matching LVGL exactly
/// Each primitive is in its own file to keep code organized

#[cfg(feature = "std")]
pub mod rectangle;
#[cfg(feature = "std")]
pub mod triangle;
#[cfg(feature = "std")]
pub mod line;
#[cfg(feature = "std")]
pub mod arc;
#[cfg(feature = "std")]
pub mod label;
#[cfg(feature = "std")]
pub mod blur;
pub mod gradient;
pub mod mask;

// Re-exports for convenience
#[cfg(feature = "std")]
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
