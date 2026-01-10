/// Rendering primitives
/// 
/// This module contains all drawing primitives:
/// - Rectangle fills
/// - Circles
/// - Lines
/// - Arcs
/// - Text (TODO)
/// - Images (TODO)

pub mod rectangle;
pub mod line;
pub mod arc;
pub mod border;
pub mod triangle;
pub mod box_shadow;

pub use rectangle::Fill;
pub use line::Line;
pub use arc::{Arc, Circle};
pub use border::{Border, BorderSide};
pub use triangle::Triangle;
pub use box_shadow::BoxShadow;
