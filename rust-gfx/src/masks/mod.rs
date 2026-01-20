//! LVGL-compatible software rendering masks.
//! This module ports the relevant logic from lv_draw_sw_mask.c.

use crate::types::Opa;

pub use angle::AngleMask;
pub use fade::FadeMask;
pub use line::{LineMask, LineSide};
pub use map::MapMask;
pub use radius::RadiusMask;

mod angle;
mod fade;
mod line;
mod map;
mod radius;

/// Result of applying masks to a scanline.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MaskResult {
    Transparent,
    FullCover,
    Changed,
}

/// Mask reference used when applying multiple masks.
pub enum MaskRef<'a> {
    Line(&'a LineMask),
    Angle(&'a AngleMask),
    Radius(&'a RadiusMask),
    Fade(&'a FadeMask),
    Map(&'a MapMask<'a>),
}

/// Apply multiple masks to a scanline buffer.
pub fn apply_masks(
    masks: &[MaskRef<'_>],
    mask_buf: &mut [Opa],
    abs_x: i32,
    abs_y: i32,
) -> MaskResult {
    if masks.is_empty() {
        return MaskResult::FullCover;
    }

    let mut changed = false;
    for mask in masks {
        let res = match mask {
            MaskRef::Line(line) => line.apply(mask_buf, abs_x, abs_y),
            MaskRef::Angle(angle) => angle.apply(mask_buf, abs_x, abs_y),
            MaskRef::Radius(radius) => radius.apply(mask_buf, abs_x, abs_y),
            MaskRef::Fade(fade) => fade.apply(mask_buf, abs_x, abs_y),
            MaskRef::Map(map) => map.apply(mask_buf, abs_x, abs_y),
        };
        match res {
            MaskResult::Transparent => return MaskResult::Transparent,
            MaskResult::FullCover => {}
            MaskResult::Changed => changed = true,
        }
    }

    if changed {
        MaskResult::Changed
    } else {
        MaskResult::FullCover
    }
}

#[inline]
pub(crate) fn mask_mix(mask_act: Opa, mask_new: Opa) -> Opa {
    if mask_act >= 255 {
        return mask_new;
    }
    if mask_act == 0 {
        return 0;
    }
    let prod = (mask_act as u32) * (mask_new as u32);
    ((prod * 0x8081) >> 23) as Opa
}

#[inline]
pub(crate) fn clamp_i32(min_v: i32, val: i32, max_v: i32) -> i32 {
    core::cmp::min(core::cmp::max(val, min_v), max_v)
}
