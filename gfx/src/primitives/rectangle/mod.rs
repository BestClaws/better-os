extern crate alloc;
mod draw;

pub use draw::{reset_temp_allocation_peak, temp_allocation_peak_bytes};

use crate::primitives::features::fill::FillStyle;
use crate::primitives::features::stroke::StrokeStyle;
use zeno::{Bounds, Point};

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

impl Edge {
    #[inline]
    fn index(self) -> usize {
        match self {
            Edge::Top => 0,
            Edge::Right => 1,
            Edge::Bottom => 2,
            Edge::Left => 3,
        }
    }
}

impl<'a> Rectangle<'a> {
    #[inline]
    pub fn new() -> Self {
        let zero = Point::new(0.0, 0.0);
        let full_clip = Bounds::new(
            Point::new(i32::MIN as f32, i32::MIN as f32),
            Point::new(i32::MAX as f32, i32::MAX as f32),
        );

        Self {
            area: Bounds::new(zero, zero),
            fill: FillStyle::default(),
            edges: [None, None, None, None],
            clip: full_clip,
            corner_radii: [CornerRadius::default(); 4],
        }
    }

    #[inline]
    pub fn bounds(mut self, bounds: Bounds) -> Self {
        self.area = bounds;
        self
    }

    #[inline]
    pub fn corner_radii(mut self, radius: CornerRadius) -> Self {
        self.corner_radii = [radius; 4];
        self
    }

    #[inline]
    pub fn corner_radii_each(mut self, radii: [CornerRadius; 4]) -> Self {
        self.corner_radii = radii;
        self
    }

    #[inline]
    pub fn fill(mut self, fill: FillStyle<3>) -> Self {
        self.fill = fill;
        self
    }

    #[inline]
    pub fn edge(mut self, edge: Edge, style: StrokeStyle<'a, 2>) -> Self {
        self.edges[edge.index()] = Some(style);
        self
    }

    #[inline]
    pub fn clear_edge(mut self, edge: Edge) -> Self {
        self.edges[edge.index()] = None;
        self
    }

    #[inline]
    pub fn clip(mut self, clip: Bounds) -> Self {
        self.clip = clip;
        self
    }

    #[inline]
    pub fn build(self) -> Self {
        self
    }
}
