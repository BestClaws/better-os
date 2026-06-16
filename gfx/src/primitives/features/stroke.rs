use crate::colors::Color;
use crate::primitives::features::gradient::{Gradient, GradientStop};

#[derive(Clone)]
pub struct StrokeStyle<'a, const GS: usize> {
    pub color: StrokeColor<GS>,
    pub stroke: zeno::Stroke<'a>,
}

#[derive(Clone)]
pub enum StrokeColor<const GS: usize> {
    Solid(Color),
    Gradient(Gradient<GS>),
}

impl<'a, const GS: usize> StrokeStyle<'a, GS> {
    #[inline]
    pub fn transparent() -> Self {
        Self::default()
    }

    #[inline]
    pub fn from_stroke(stroke: zeno::Stroke<'a>) -> Self {
        let mut style = Self::default();
        style.stroke = stroke;
        style
    }

    #[inline]
    pub fn solid(mut self, color: Color) -> Self {
        self.color = StrokeColor::Solid(color);
        self
    }

    #[inline]
    pub fn horizontal_gradient(mut self, stops: [(Color, u8); GS]) -> Self {
        self.color = StrokeColor::Gradient(Gradient::Horizontal(GradientStop::new(stops)));
        self
    }

    #[inline]
    pub fn vertical_gradient(mut self, stops: [(Color, u8); GS]) -> Self {
        self.color = StrokeColor::Gradient(Gradient::Vertical(GradientStop::new(stops)));
        self
    }

    #[inline]
    pub fn finalize(self) -> Self {
        self
    }
}

impl<'a, const GS: usize> Default for StrokeStyle<'a, GS> {
    fn default() -> Self {
        let mut stroke = zeno::Stroke::default();
        stroke.width = 0.0;

        Self {
            color: StrokeColor::Solid(Color::rgba(0, 0, 0, 0)),
            stroke,
        }
    }
}
