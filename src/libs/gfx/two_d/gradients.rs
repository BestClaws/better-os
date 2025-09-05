use crate::libs::gfx::two_d::types::{Point, Rect, Size, Rgb565, Rgba8888};
use crate::libs::gfx::two_d::raster::Rasterizer;

#[derive(Clone, Copy, Debug)]
pub struct LinearGradient { pub start: Point, pub end: Point, pub start_color: Rgb565, pub end_color: Rgb565 }
#[derive(Clone, Copy, Debug)]
pub struct RadialGradient { pub center: Point, pub radius: u32, pub inner_color: Rgb565, pub outer_color: Rgb565 }

fn lerp_rgb565(a: Rgb565, b: Rgb565, t: f32) -> Rgb565 { let at = (t * 255.0).clamp(0.0, 255.0) as u8; b.blend_over(a, at) }

impl LinearGradient {
    pub fn sample(&self, p: Point) -> Rgb565 {
        let dx = (self.end.x - self.start.x) as i64; let dy = (self.end.y - self.start.y) as i64; let len2 = (dx * dx + dy * dy).max(1);
        let t_num = ((p.x - self.start.x) as i64 * dx + (p.y - self.start.y) as i64 * dy).clamp(0, len2);
        let t = (t_num as f32) / (len2 as f32); lerp_rgb565(self.start_color, self.end_color, t)
    }
}

impl RadialGradient { pub fn sample(&self, p: Point) -> Rgb565 { let dx = p.x - self.center.x; let dy = p.y - self.center.y; let d2 = (dx as i64 * dx as i64 + dy as i64 * dy as i64) as f32; let r = (self.radius as f32).max(1.0); let t = (d2.sqrt() / r).clamp(0.0, 1.0); lerp_rgb565(self.inner_color, self.outer_color, t) } }

pub fn fill_rect_linear_gradient(r: &mut dyn Rasterizer, rect: Rect, grad: &LinearGradient) { let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height())); if let Some(rc) = rect.intersection(&clip) { for y in rc.top_left.y..=rc.bottom() { for x in rc.top_left.x..=rc.right() { let c = grad.sample(Point::new(x, y)); r.set_pixel(x, y, c); } } } }
pub fn fill_rect_radial_gradient(r: &mut dyn Rasterizer, rect: Rect, grad: &RadialGradient) { let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height())); if let Some(rc) = rect.intersection(&clip) { for y in rc.top_left.y..=rc.bottom() { for x in rc.top_left.x..=rc.right() { let c = grad.sample(Point::new(x, y)); r.set_pixel(x, y, c); } } } }
pub fn fill_rect_rgba(r: &mut dyn Rasterizer, rect: Rect, color: Rgba8888) { let clip = Rect::new(Point::zero(), Size::new(r.width(), r.height())); if let Some(rc) = rect.intersection(&clip) { let rgb = color.to_rgb565(); for y in rc.top_left.y..=rc.bottom() { for x in rc.top_left.x..=rc.right() { r.blend_pixel(x, y, rgb, color.a); } } } }

use micromath::F32Ext;