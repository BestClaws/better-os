extern crate alloc;
mod draw;

use crate::primitives::features::fill::FillStyle;
use crate::primitives::features::stroke::StrokeStyle;
use zeno::Bounds;

#[derive(Copy, Clone, Default)]
pub struct CornerRadius {
    pub horizontal: f32,
    pub vertical: f32,
}

impl CornerRadius {
    pub const fn new(horizontal: f32, vertical: f32) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

pub struct Rectangle<'a> {
    /// Rectangle bounds (x, y, width, height)
    pub area: Bounds,
    /// Fill style (solid color or gradient)
    pub fill: FillStyle<3>,
    /// Per-edge borders: [top, right, bottom, left]
    /// None = no border on that edge
    pub edges: [Option<StrokeStyle<'a, 2>>; 4],
    /// Clipping bounds (coordinates outside this are not rendered)
    pub clip: Bounds,
    /// Corner radii: [top-left, top-right, bottom-right, bottom-left]
    pub corner_radii: [CornerRadius; 4],
}
