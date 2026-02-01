use crate::colors::Color;
use crate::primitives::features::gradient::Gradient;

#[derive(Clone)]
pub struct StrokeStyle<'a, const GS: usize> {
    pub color: StrokeColor<GS>,
    pub stroke: zeno::Stroke<'a>
}



#[derive(Clone)]
pub enum StrokeColor<const GS: usize> {
    Solid(Color),
    Gradient(Gradient<GS>),
}

