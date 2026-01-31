pub mod math;
pub mod model;
pub mod render;

pub use math::{Mat4, Quaternion, Vec2, Vec3, Vec4};
pub use model::{load_glb, Material, Mesh, ModelError, Node, Scene, Texture, Vertex};
pub use render::{render_scene, Camera, RenderOptions, ShadingMode};
