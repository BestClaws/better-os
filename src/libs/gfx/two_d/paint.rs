#![no_std]

use crate::libs::gfx::two_d::types::{Point, Rgba8888};
use crate::libs::gfx::two_d::gradients::{LinearGradient, RadialGradient};

/// Generic paint sampler: returns RGBA8888 color at a given pixel
pub trait PixelSampler {
    fn sample(&self, x: i32, y: i32) -> Rgba8888;
}

impl PixelSampler for Rgba8888 {
    #[inline(always)]
    fn sample(&self, _x: i32, _y: i32) -> Rgba8888 { *self }
}

impl PixelSampler for LinearGradient {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> Rgba8888 { self.sample(Point::new(x, y)) }
}

impl PixelSampler for RadialGradient {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> Rgba8888 { self.sample(Point::new(x, y)) }
}

/// Universal brush used for fills and strokes.
#[derive(Clone, Copy, Debug)]
pub enum Brush {
    Solid(Rgba8888),
    Rgba(Rgba8888), // alias of Solid; kept for clarity
    Linear(LinearGradient),
    Radial(RadialGradient),
}

impl PixelSampler for Brush {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> Rgba8888 {
        match self {
            Brush::Solid(c) => *c,
            Brush::Rgba(c) => *c,
            Brush::Linear(g) => g.sample(Point::new(x, y)),
            Brush::Radial(g) => g.sample(Point::new(x, y)),
        }
    }
}

impl Brush {
    #[inline(always)]
    pub fn solid(c: Rgba8888) -> Self { Brush::Solid(c) }
    #[inline(always)]
    pub fn rgba(c: Rgba8888) -> Self { Brush::Rgba(c) }
    #[inline(always)]
    pub fn linear(g: LinearGradient) -> Self { Brush::Linear(g) }
    #[inline(always)]
    pub fn radial(g: RadialGradient) -> Self { Brush::Radial(g) }
}


