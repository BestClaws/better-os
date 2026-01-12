// file: src/shapes/mod.rs

mod text;

pub use text::Text;

// Re-export Shape trait from rust-gfx
pub use rust_gfx::Shape;