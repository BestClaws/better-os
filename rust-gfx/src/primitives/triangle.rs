use crate::color::Rgba8888;
use crate::masks::{apply_masks, LineMask, LineSide, MaskRef, MaskResult};
use crate::primitives::gradient::*;
use crate::types::*;
use crate::Rasterizer;
/// Triangle drawing matching LVGL's lv_draw_triangle
use alloc::vec;
use alloc::vec::Vec;

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
pub fn draw_triangle<R: Rasterizer>(rast: &mut R, dsc: &TriangleDsc) {
    // Sort points: p[0] has smallest y, p[1] has largest y, p[2] is middle
    // LVGL does: sort so p[0].y <= p[2].y <= p[1].y
    let mut p = [dsc.p1, dsc.p2, dsc.p3];

    // Bubble sort to get p[0] as top (smallest y)
    if p[0].y > p[2].y {
        p.swap(0, 2);
    }
    if p[0].y > p[1].y {
        p.swap(0, 1);
    }
    if p[1].y < p[2].y {
        p.swap(1, 2);
    }

    // Determine if p[2] is on the right side of the p[0]-p[1] line
    // Cross product: (p[1] - p[0]) × (p[2] - p[0])
    let right = ((p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[1].y - p[0].y) * (p[2].x - p[0].x)) < 0;

    // Create three line masks matching LVGL's approach
    let mask_left = LineMask::from_points(
        p[0],
        p[1],
        if right {
            LineSide::Right
        } else {
            LineSide::Left
        },
    );

    let mask_right = LineMask::from_points(
        p[0],
        p[2],
        if right {
            LineSide::Left
        } else {
            LineSide::Right
        },
    );

    let mask_bottom = if p[1].y == p[2].y {
        // Bottom edge is horizontal
        LineMask::from_points(p[1], p[2], LineSide::Top)
    } else {
        // Bottom edge
        LineMask::from_points(
            p[1],
            p[2],
            if right {
                LineSide::Left
            } else {
                LineSide::Right
            },
        )
    };

    // Get bounding box
    let min_x = p[0].x.min(p[1].x).min(p[2].x);
    let max_x = p[0].x.max(p[1].x).max(p[2].x);
    let min_y = p[0].y.min(p[1].y).min(p[2].y);
    let max_y = p[0].y.max(p[1].y).max(p[2].y);

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let area_w = (max_x - min_x + 1) as usize;

    let has_grad = dsc.grad.dir != GradDir::None;

    // Mask buffer for scanline
    let mut mask_buf = vec![255u8; area_w];

    // Draw triangle scanline by scanline (LVGL approach)
    for y in min_y..=max_y {
        // Reset mask buffer to full opacity
        mask_buf.fill(255);

        // Apply all three masks to the scanline
        let masks = vec![
            MaskRef::Line(&mask_left),
            MaskRef::Line(&mask_right),
            MaskRef::Line(&mask_bottom),
        ];
        let mask_res = apply_masks(&masks, &mut mask_buf, min_x, y);

        if mask_res == MaskResult::Transparent {
            continue;
        }

        let mask_full_cover = mask_res == MaskResult::FullCover;

        // Blend pixels with mask applied
        for i in 0..area_w {
            let x = min_x + i as i32;
            let mut mask_opa = if mask_full_cover {
                OPA_COVER
            } else {
                mask_buf[i]
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
                let rel_x = x - min_x;
                let rel_y = y - min_y;
                let (grad_color, grad_opa) = gradient_get_color(
                    &dsc.grad,
                    rel_x,
                    rel_y,
                    width,
                    height,
                    width / 2,
                    height / 2,
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

            // Write all pixels for non-premultiplied alpha (even if final_opa=0)
            rast.blend_pixel(x, y, final_color, final_opa);
        }
    }

    rast.mark_dirty(min_x, min_y, max_x + 1, max_y + 1);
}
