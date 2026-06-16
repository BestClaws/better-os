use crate::colors::Color;

#[derive(Clone)]
pub enum Gradient<const GS: usize> {
    Horizontal(GradientStop<GS>),
    Vertical(GradientStop<GS>),
}

#[derive(Clone)]
pub struct GradientStop<const GS: usize>(pub [(Color, u8); GS]);

impl<const GS: usize> GradientStop<GS> {
    pub const fn new(stops: [(Color, u8); GS]) -> Self {
        Self(stops)
    }

    pub const fn as_array(&self) -> &[(Color, u8); GS] {
        &self.0
    }

    pub fn apply_opacity(&mut self, opacity: u8) {
        if opacity >= 255 {
            return;
        }

        for (color, _) in self.0.iter_mut() {
            *color = color.scale_alpha(opacity);
        }
    }
}
