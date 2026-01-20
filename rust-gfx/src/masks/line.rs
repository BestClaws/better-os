use crate::math::{trigo_cos, trigo_sin};
use crate::types::{Opa, Point};
use super::{mask_mix, MaskResult};
use core::mem;

/// Which side of the line to keep when masking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LineSide {
    Left,
    Right,
    Top,
    Bottom,
}

/// Software line mask descriptor (port of lv_draw_sw_mask_line_param_t).
#[derive(Clone, Debug)]
pub struct LineMask {
    p1: Point,
    p2: Point,
    side: LineSide,
    origo: Point,
    xy_steep: i32,
    yx_steep: i32,
    steep: i32,
    spx: i32,
    flat: bool,
    inv: bool,
}

impl LineMask {
    /// Create mask from two points keeping the specified side.
    pub fn from_points(mut p1: Point, mut p2: Point, side: LineSide) -> Self {
        if p1.y == p2.y && matches!(side, LineSide::Bottom) {
            p1.y -= 1;
            p2.y -= 1;
        }

        if p1.y > p2.y {
            mem::swap(&mut p1, &mut p2);
        }

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let flat = dx.abs() > dy.abs();

        let mut xy_steep = 0;
        let mut yx_steep = 0;

        if flat {
            if dx != 0 {
                let m = ((1i64 << 20) / dx as i64) as i64;
                yx_steep = ((m * dy as i64) >> 10) as i32;
            }
            if dy != 0 {
                let m = ((1i64 << 20) / dy as i64) as i64;
                xy_steep = ((m * dx as i64) >> 10) as i32;
            }
        } else {
            if dy != 0 {
                let m = ((1i64 << 20) / dy as i64) as i64;
                xy_steep = ((m * dx as i64) >> 10) as i32;
            }
            if dx != 0 {
                let m = ((1i64 << 20) / dx as i64) as i64;
                yx_steep = ((m * dy as i64) >> 10) as i32;
            }
        }

        let steep = if flat { yx_steep } else { xy_steep };
        let mut inv = false;

        match side {
            LineSide::Left => inv = false,
            LineSide::Right => inv = true,
            LineSide::Top => inv = steep > 0,
            LineSide::Bottom => inv = steep <= 0,
        }

        let mut spx = steep >> 2;
        if steep < 0 {
            spx = -spx;
        }

        Self {
            p1,
            p2,
            side,
            origo: p1,
            xy_steep,
            yx_steep,
            steep,
            spx,
            flat,
            inv,
        }
    }

    /// Create mask from vertex and angle in degrees.
    pub fn from_angle(p: Point, mut angle: i32, side: LineSide) -> Self {
        if angle > 180 {
            angle -= 180;
        }
        let p2x = (trigo_cos(angle) >> 5) + p.x;
        let p2y = (trigo_sin(angle) >> 5) + p.y;
        Self::from_points(p, Point::new(p2x, p2y), side)
    }

    /// Return the normalized XY steepness used for angle masks.
    pub(crate) fn xy_steep(&self) -> i32 {
        self.xy_steep
    }

    /// Return the normalized YX steepness used for angle masks.
    pub(crate) fn yx_steep(&self) -> i32 {
        self.yx_steep
    }

    fn apply_flat(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;
        let mut y_at_x = ((self.yx_steep as i64 * abs_x as i64) >> 10) as i32;

        if self.yx_steep > 0 {
            if y_at_x > abs_y {
                return if self.inv {
                    MaskResult::FullCover
                } else {
                    MaskResult::Transparent
                };
            }
        } else if y_at_x < abs_y {
            return if self.inv {
                MaskResult::FullCover
            } else {
                MaskResult::Transparent
            };
        }

        y_at_x = ((self.yx_steep as i64 * (abs_x + len) as i64) >> 10) as i32;
        if self.yx_steep > 0 {
            if y_at_x < abs_y {
                return if self.inv {
                    MaskResult::Transparent
                } else {
                    MaskResult::FullCover
                };
            }
        } else if y_at_x > abs_y {
            return if self.inv {
                MaskResult::Transparent
            } else {
                MaskResult::FullCover
            };
        }

        let xe = if self.yx_steep > 0 {
            ((abs_y * 256) as i64 * self.xy_steep as i64) >> 10
        } else {
            (((abs_y + 1) * 256) as i64 * self.xy_steep as i64) >> 10
        };

        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;
        let mut px_h = if xef == 0 {
            255
        } else {
            255 - (((255 - xef) * self.spx) >> 8)
        };
        let mut k = xei - abs_x;

        if xef != 0 {
            if k >= 0 && k < len {
                let mut m = 255 - (((255 - xef) * (255 - px_h)) >> 9);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            k += 1;
        }

        while px_h > self.spx {
            if k >= 0 && k < len {
                let mut m = px_h - (self.spx >> 1);
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            px_h -= self.spx;
            k += 1;
            if k >= len {
                break;
            }
        }

        if k < len && k >= 0 {
            let x_inters = ((px_h as i64 * self.xy_steep as i64) >> 10) as i32;
            let mut m = (x_inters * px_h) >> 9;
            if self.yx_steep < 0 {
                m = 255 - m;
            }
            if self.inv {
                m = 255 - m;
            }
            mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
        }

        if self.inv {
            let mut k = xei - abs_x;
            if k > len {
                return MaskResult::Transparent;
            }
            if k > 0 {
                mask_buf[..k as usize].fill(0);
            }
        } else {
            k += 1;
            if k < 0 {
                return MaskResult::Transparent;
            }
            if k < len {
                mask_buf[k as usize..].fill(0);
            }
        }

        MaskResult::Changed
    }

    fn apply_steep(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;
        let mut x_at_y = ((self.xy_steep as i64 * abs_y as i64) >> 10) as i32;
        if self.xy_steep > 0 {
            x_at_y += 1;
        }
        if x_at_y < abs_x {
            return if self.inv {
                MaskResult::FullCover
            } else {
                MaskResult::Transparent
            };
        }

        x_at_y = ((self.xy_steep as i64 * abs_y as i64) >> 10) as i32;
        if x_at_y > abs_x + len {
            return if self.inv {
                MaskResult::Transparent
            } else {
                MaskResult::FullCover
            };
        }

        let xs = ((abs_y * 256) as i64 * self.xy_steep as i64) >> 10;
        let mut xsi = (xs >> 8) as i32;
        let mut xsf = (xs & 0xFF) as i32;

        let xe = (((abs_y + 1) * 256) as i64 * self.xy_steep as i64) >> 10;
        let xei = (xe >> 8) as i32;
        let xef = (xe & 0xFF) as i32;

        let mut k = xsi - abs_x;
        if xsi != xei && self.xy_steep < 0 && xsf == 0 {
            xsf = 0xFF;
            xsi = xei;
            k -= 1;
        }

        if xsi == xei {
            if k >= 0 && k < len {
                let mut m = (xsf + xef) >> 1;
                if self.inv {
                    m = 255 - m;
                }
                mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
            }
            k += 1;

            if self.inv {
                let mut k = xsi - abs_x;
                if k >= len {
                    return MaskResult::Transparent;
                }
                if k > 0 {
                    mask_buf[..k as usize].fill(0);
                }
            } else {
                if k > len {
                    k = len;
                }
                if k == 0 {
                    return MaskResult::Transparent;
                }
                if k < len {
                    mask_buf[k as usize..].fill(0);
                }
            }
        } else {
            if self.xy_steep < 0 {
                let y_inters = ((xsf * (-self.yx_steep)) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = (y_inters * xsf) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k -= 1;

                let x_inters = (((255 - y_inters) * (-self.xy_steep)) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = 255 - (((255 - y_inters) * x_inters) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 2;

                if self.inv {
                    let mut k = xsi - abs_x - 1;
                    if k > len {
                        k = len;
                    }
                    if k > 0 {
                        mask_buf[..k as usize].fill(0);
                    }
                } else {
                    if k > len {
                        return MaskResult::FullCover;
                    }
                    if k >= 0 {
                        mask_buf[k as usize..].fill(0);
                    }
                }
            } else {
                let y_inters = (((255 - xsf) * self.yx_steep) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = 255 - ((y_inters * (255 - xsf)) >> 9);
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 1;

                let x_inters = (((255 - y_inters) * self.xy_steep) >> 10) as i32;
                if k >= 0 && k < len {
                    let mut m = ((255 - y_inters) * x_inters) >> 9;
                    if self.inv {
                        m = 255 - m;
                    }
                    mask_buf[k as usize] = mask_mix(mask_buf[k as usize], m as Opa);
                }
                k += 1;

                if self.inv {
                    let mut k = xsi - abs_x;
                    if k > len {
                        return MaskResult::Transparent;
                    }
                    if k > 0 {
                        mask_buf[..k as usize].fill(0);
                    }
                } else {
                    if k > len {
                        k = len;
                    }
                    if k == 0 {
                        return MaskResult::Transparent;
                    }
                    if k < len {
                        mask_buf[k as usize..].fill(0);
                    }
                }
            }
        }

        MaskResult::Changed
    }

    /// Apply line mask to a scanline buffer.
    pub fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        let len = mask_buf.len() as i32;

        let rel_y = abs_y - self.origo.y;
        let rel_x = abs_x - self.origo.x;

        if self.steep == 0 {
            if self.flat {
                match self.side {
                    LineSide::Left | LineSide::Right => return MaskResult::FullCover,
                    LineSide::Top if rel_y < 0 => return MaskResult::FullCover,
                    LineSide::Bottom if rel_y > 0 => return MaskResult::FullCover,
                    _ => return MaskResult::Transparent,
                }
            } else {
                match self.side {
                    LineSide::Top | LineSide::Bottom => return MaskResult::FullCover,
                    LineSide::Right if rel_x > 0 => return MaskResult::FullCover,
                    LineSide::Left => {
                        if rel_x + len < 0 {
                            return MaskResult::FullCover;
                        }
                        let k = -rel_x;
                        if k < 0 {
                            return MaskResult::Transparent;
                        }
                        if k < len {
                            mask_buf[k as usize..].fill(0);
                            return MaskResult::Changed;
                        }
                        return MaskResult::Changed;
                    }
                    LineSide::Right => {
                        if rel_x + len < 0 {
                            return MaskResult::Transparent;
                        }
                        let mut k = -rel_x;
                        if k < 0 {
                            k = 0;
                        }
                        if k >= len {
                            return MaskResult::Transparent;
                        }
                        mask_buf[..k as usize].fill(0);
                        return MaskResult::Changed;
                    }
                    _ => return MaskResult::FullCover,
                }
            }
        }

        if self.flat {
            self.apply_flat(mask_buf, rel_x, rel_y)
        } else {
            self.apply_steep(mask_buf, rel_x, rel_y)
        }
    }
}
