/// Triangle drawing matching LVGL's lv_draw_triangle
use crate::canvas::Canvas;
use crate::color::Rgba8888;
use crate::types::*;
use crate::primitives::{gradient::*, mask::TriangleMask};

/// Triangle descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct TriangleDsc {
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
    pub color: Rgba8888,
    pub opa: Opa,
    pub grad: Gradient,
}

impl TriangleDsc {
    pub fn new(p1: Point, p2: Point, p3: Point) -> Self {
        Self {
            p1,
            p2,
            p3,
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            grad: Gradient::none(),
        }
    }
}

/// Draw a filled triangle
pub fn draw_triangle(canvas: &mut Canvas, dsc: &TriangleDsc) {
    let mask = TriangleMask::new(
        dsc.p1.x, dsc.p1.y,
        dsc.p2.x, dsc.p2.y,
        dsc.p3.x, dsc.p3.y,
    );

    // Get bounding box
    let min_x = dsc.p1.x.min(dsc.p2.x).min(dsc.p3.x);
    let max_x = dsc.p1.x.max(dsc.p2.x).max(dsc.p3.x);
    let min_y = dsc.p1.y.min(dsc.p2.y).min(dsc.p3.y);
    let max_y = dsc.p1.y.max(dsc.p2.y).max(dsc.p3.y);

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let cx = (min_x + max_x) / 2;
    let cy = (min_y + max_y) / 2;

    let has_grad = dsc.grad.dir != GradDir::None;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if mask.contains_point(x, y) {
                let (color, grad_opa) = if has_grad {
                    let rel_x = x - min_x;
                    let rel_y = y - min_y;
                    gradient_get_color(&dsc.grad, rel_x, rel_y, width, height, width / 2, height / 2)
                } else {
                    (dsc.color, OPA_COVER)
                };

                let opa = ((dsc.opa as u32 * grad_opa as u32) / 255) as Opa;
                canvas.blend_pixel(x, y, color, opa);
            }
        }
    }
}
