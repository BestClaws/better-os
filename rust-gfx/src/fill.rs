use crate::color::Rgba8888;

#[derive(Copy, Clone)]
pub enum Fill {
    Solid(Rgba8888),
    RadialGradient { inner: Rgba8888, outer: Rgba8888 },
    LinearGradientH { start: Rgba8888, end: Rgba8888 },
    LinearGradientV { start: Rgba8888, end: Rgba8888 },
}
