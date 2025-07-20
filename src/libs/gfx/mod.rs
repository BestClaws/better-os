use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point, Size},
};

pub mod math;
pub mod model;
pub mod render;

pub use math::{Quaternion, Vec3};
pub use model::{Model, StlError, parse_binary_stl};
pub use render::{draw_model, RenderOptions};