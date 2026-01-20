use crate::color::Rgba8888;
use crate::masks::{LineMask, LineSide, MaskRef, MaskResult};
use crate::primitives::common::{apply_scanline_masks, MaskBuffer};
use crate::primitives::gradient::*;
use crate::types::*;
use crate::Rasterizer;
/// Triangle drawing matching LVGL's lv_draw_triangle

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

#[inline]
fn triangle_opa_mix(a: Opa, b: Opa) -> Opa {
    let prod = (a as u32) * (b as u32);
    (((prod * 0x8081) >> 23) & 0xFF) as Opa
}

/// Draw a filled triangle with anti-aliased edges using LVGL's 3-line-mask approach
pub fn draw_triangle<R>(rast: &mut R, dsc: &TriangleDsc)
where
    R: Rasterizer,
{
    let mut p = [dsc.p1, dsc.p2, dsc.p3];

    // Match LVGL's preference for vertical edges when ordering vertices
    if dsc.p1.x == dsc.p2.x {
        p = [dsc.p1, dsc.p2, dsc.p3];
    } else if dsc.p1.x == dsc.p3.x {
        p = [dsc.p1, dsc.p3, dsc.p2];
    } else if dsc.p2.x == dsc.p3.x {
        p = [dsc.p2, dsc.p3, dsc.p1];
    }

    // Ensure p[0] is the top vertex, p[1] the bottom, p[2] the middle (LVGL ordering)
    if p[0].y > p[2].y {
        p.swap(0, 2);
    }
    if p[0].y > p[1].y {
        p.swap(0, 1);
    }
    if p[1].y < p[2].y {
        p.swap(1, 2);
    }

    if rast.width() == 0 || rast.height() == 0 {
        return;
    }

    let min_x = p.iter().map(|pt| pt.x).min().unwrap_or(0);
    let max_x = p.iter().map(|pt| pt.x).max().unwrap_or(-1);
    let min_y = p.iter().map(|pt| pt.y).min().unwrap_or(0);
    let max_y = p.iter().map(|pt| pt.y).max().unwrap_or(-1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let tri_area = Area::new(min_x, min_y, max_x, max_y);
    let surface_area = Area::new(0, 0, rast.width_i32() - 1, rast.height_i32() - 1);
    let Some(draw_area) = tri_area.intersect(&surface_area) else {
        return;
    };

    let right_side = ((p[1].x - p[0].x) * (p[2].y - p[0].y)
        - (p[1].y - p[0].y) * (p[2].x - p[0].x))
        < 0;

    let mask_left = LineMask::from_points(
        p[0],
        p[1],
        if right_side {
            LineSide::Right
        } else {
            LineSide::Left
        },
    );
    let mask_right = LineMask::from_points(
        p[0],
        p[2],
        if right_side {
            LineSide::Left
        } else {
            LineSide::Right
        },
    );
    let mask_bottom = if p[1].y == p[2].y {
        LineMask::from_points(p[1], p[2], LineSide::Top)
    } else {
        LineMask::from_points(
            p[1],
            p[2],
            if right_side {
                LineSide::Left
            } else {
                LineSide::Right
            },
        )
    };

    let mut mask_buffer = MaskBuffer::default();
    let mask_refs = [
        MaskRef::Line(&mask_left),
        MaskRef::Line(&mask_right),
        MaskRef::Line(&mask_bottom),
    ];

    let span_width = draw_area.width();
    if span_width <= 0 {
        return;
    }

    let span_width_usize = span_width as usize;
    let tri_width = tri_area.width().max(1);
    let tri_height = tri_area.height().max(1);
    let has_grad = dsc.grad.dir != GradDir::None;

    for y in draw_area.y1..=draw_area.y2 {
        let (mask_res, mask_values) = apply_scanline_masks(
            &mut mask_buffer,
            span_width_usize,
            draw_area.x1,
            y,
            &mask_refs,
        );

        if mask_res == MaskResult::Transparent {
            continue;
        }

        let mask_full_cover = mask_res == MaskResult::FullCover
            && mask_values.iter().all(|&mask| mask == OPA_COVER);

        if !has_grad && mask_full_cover && dsc.opa == OPA_COVER {
            rast.blend_hspan_with(draw_area.x1, y, span_width, |_| (dsc.color, OPA_COVER));
            continue;
        }

        for (index, &raw_mask) in mask_values.iter().enumerate() {
            let x = draw_area.x1 + index as i32;
            let mut mask_opa = if mask_full_cover {
                OPA_COVER
            } else {
                raw_mask
            };

            if mask_opa <= 2 {
                mask_opa = OPA_TRANSP;
            } else if mask_opa >= 253 {
                mask_opa = OPA_COVER;
            }

            let mut base_opa = dsc.opa;
            let mut final_color = dsc.color;
            let mut use_mask = !mask_full_cover || dsc.opa < OPA_COVER;

            if has_grad {
                let rel_x = x - tri_area.x1;
                let rel_y = y - tri_area.y1;
                let (grad_color, grad_opa) = gradient_get_color(
                    &dsc.grad,
                    rel_x,
                    rel_y,
                    tri_width,
                    tri_height,
                    tri_width / 2,
                    tri_height / 2,
                );

                final_color = grad_color;

                match dsc.grad.dir {
                    GradDir::Ver => {
                        base_opa = if dsc.opa < OPA_COVER {
                            opa_mix(grad_opa, dsc.opa)
                        } else {
                            grad_opa
                        };
                        use_mask = !mask_full_cover;
                    }
                    GradDir::Hor => {
                        if mask_full_cover {
                            mask_opa = grad_opa;
                            use_mask = true;
                        } else if grad_opa < OPA_COVER {
                            mask_opa = opa_mix(mask_opa, grad_opa);
                        }
                        base_opa = dsc.opa;
                    }
                    _ => {
                        base_opa = if dsc.opa < OPA_COVER {
                            opa_mix(grad_opa, dsc.opa)
                        } else {
                            grad_opa
                        };
                        if mask_full_cover {
                            mask_opa = grad_opa;
                            use_mask = true;
                        } else if grad_opa < OPA_COVER {
                            mask_opa = opa_mix(mask_opa, grad_opa);
                        }
                    }
                }
            }

            let final_opa = if use_mask {
                if base_opa >= OPA_COVER {
                    triangle_opa_mix(base_opa, mask_opa)
                } else {
                    opa_mix(base_opa, mask_opa)
                }
            } else {
                base_opa
            };

            rast.blend_pixel(x, y, final_color, final_opa);
        }
    }
}
