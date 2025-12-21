// file: src/shapes/mod.rs

// file: src/shapes/mod.rs
// (minor update to re-export color for convenience if desired)

mod arc;
mod circle;
mod rounded_rect;
mod line;
mod text;

pub use arc::Arc;
pub use circle::Circle;
pub use rounded_rect::RoundedRect;
pub use line::Line;
pub use text::Text;
pub trait Shape {
    fn draw<R: super::Rasterizer>(&self, rasterizer: &mut R);
}