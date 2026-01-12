// file: src/shapes/mod.rs

mod arc;
mod circle;
mod line;
mod rounded_rect;

pub use arc::Arc;
pub use circle::Circle;
pub use line::Line;
pub use rounded_rect::RoundedRect;

pub trait Shape {
    fn draw<R: super::Rasterizer>(&self, rasterizer: &mut R);
}
