use crate::colors::Color;

#[derive(Clone)]
pub enum Gradient<const GS: usize> {
    Horizontal(GradientStop<GS>),
    Vertical(GradientStop<GS>),
}

#[derive(Clone)]
pub struct GradientStop<const GS: usize>(pub [(Color, u8); GS]);
