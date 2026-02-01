use crate::primitives::rectangle::Rectangle;
use crate::rasterizer::RasterTarget;

impl<'a> Rectangle<'a> {
    pub fn draw(&self, canvas: &mut dyn RasterTarget) {}
}

