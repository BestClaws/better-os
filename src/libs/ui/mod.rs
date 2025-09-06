//! Space-grade immediate-mode UI built on gfx::two_d primitives.
//! 
//! Provides:
//! - Immediate UI (ImUi) with built-in input snapshot
//! - Styles and theme
//! - Painter wrapper over `Rasterizer`/`Canvas`

pub mod style;
pub mod painter;
pub mod imui;

// Re-exports for convenience
pub use style::{Color, Stroke, Fill, CornerRadii, Theme, Palette, TextStyle};
pub use painter::Painter;
pub use imui::{ImUi, ImInput, Response};
