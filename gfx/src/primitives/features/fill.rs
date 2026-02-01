use crate::colors::Color;
use crate::primitives::features::gradient::Gradient;

pub enum FillStyle<const GS: usize> {
    Solid(Color),
    Gradient(Gradient<GS>),
    
}
