/// Logical edges of the active framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenEdge {
    Left,
    Right,
    Top,
    Bottom,
}
