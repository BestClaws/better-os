use crate::colors::Color;
use crate::primitives::features::gradient::{Gradient, GradientStop};

pub enum FillStyle<const GS: usize> {
    Solid(Color),
    Gradient(Gradient<GS>),
}

impl<const GS: usize> FillStyle<GS> {
    #[inline]
    pub fn transparent() -> Self {
        Self::Solid(Color::rgba(0, 0, 0, 0))
    }

    #[inline]
    pub fn solid(color: Color) -> Self {
        Self::Solid(color)
    }

    #[inline]
    pub fn horizontal_gradient(stops: [(Color, u8); GS]) -> Self {
        Self::Gradient(Gradient::Horizontal(GradientStop::new(stops)))
    }

    #[inline]
    pub fn vertical_gradient(stops: [(Color, u8); GS]) -> Self {
        Self::Gradient(Gradient::Vertical(GradientStop::new(stops)))
    }

    #[inline]
    pub fn with_opacity(mut self, opacity: u8) -> Self {
        if opacity >= 255 {
            return self;
        }

        match &mut self {
            FillStyle::Solid(color) => {
                *color = (*color).scale_alpha(opacity);
            }
            FillStyle::Gradient(Gradient::Horizontal(stops)) => {
                stops.apply_opacity(opacity);
            }
            FillStyle::Gradient(Gradient::Vertical(stops)) => {
                stops.apply_opacity(opacity);
            }
        }

        self
    }
}

impl<const GS: usize> Default for FillStyle<GS> {
    fn default() -> Self {
        Self::transparent()
    }
}
