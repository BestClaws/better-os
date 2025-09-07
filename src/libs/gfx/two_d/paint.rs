#![no_std]

use crate::libs::gfx::two_d::types::{Point, Rgb565, Rgba8888};
use crate::libs::gfx::two_d::gradients::{LinearGradient, RadialGradient};

/// Generic paint sampler: returns RGB565 color and alpha at a given pixel
pub trait PixelSampler {
    fn sample(&self, x: i32, y: i32) -> (Rgb565, u8);
}

impl PixelSampler for Rgb565 {
    #[inline(always)]
    fn sample(&self, _x: i32, _y: i32) -> (Rgb565, u8) { (*self, 255) }
}

impl PixelSampler for Rgba8888 {
    #[inline(always)]
    fn sample(&self, _x: i32, _y: i32) -> (Rgb565, u8) { (self.to_rgb565(), self.a) }
}

impl PixelSampler for LinearGradient {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> (Rgb565, u8) { (self.sample(Point::new(x, y)), 255) }
}

impl PixelSampler for RadialGradient {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> (Rgb565, u8) { (self.sample(Point::new(x, y)), 255) }
}

/// Universal brush used for fills and strokes.
#[derive(Clone, Copy, Debug)]
pub enum Brush {
    Solid(Rgb565),
    Rgba(Rgba8888),
    Linear(LinearGradient),
    Radial(RadialGradient),
}

impl PixelSampler for Brush {
    #[inline(always)]
    fn sample(&self, x: i32, y: i32) -> (Rgb565, u8) {
        match self {
            Brush::Solid(c) => (*c, 255),
            Brush::Rgba(c) => (c.to_rgb565(), c.a),
            Brush::Linear(g) => (g.sample(Point::new(x, y)), 255),
            Brush::Radial(g) => (g.sample(Point::new(x, y)), 255),
        }
    }
}

impl Brush {
    #[inline(always)]
    pub fn solid(c: Rgb565) -> Self { Brush::Solid(c) }
    #[inline(always)]
    pub fn rgba(c: Rgba8888) -> Self { Brush::Rgba(c) }
    #[inline(always)]
    pub fn linear(g: LinearGradient) -> Self { Brush::Linear(g) }
    #[inline(always)]
    pub fn radial(g: RadialGradient) -> Self { Brush::Radial(g) }
}


