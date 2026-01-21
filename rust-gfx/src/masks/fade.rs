use super::{mask_mix, MaskResult};
use crate::types::{Area, Opa};

/// Vertical fade mask descriptor (port of lv_draw_sw_mask_fade_param_t).
#[derive(Clone, Debug)]
pub struct FadeMask {
    coords: Area,
    opa_top: Opa,
    opa_bottom: Opa,
    y_top: i32,
    y_bottom: i32,
}

impl FadeMask {
    /// Create a fade mask with opacity interpolated between the top and bottom rows.
    pub fn new(coords: Area, opa_top: Opa, y_top: i32, opa_bottom: Opa, y_bottom: i32) -> Self {
        Self {
            coords,
            opa_top,
            opa_bottom,
            y_top,
            y_bottom,
        }
    }

    /// Apply fade mask to the provided buffer.
    pub fn apply(&self, mask_buf: &mut [Opa], abs_x: i32, abs_y: i32) -> MaskResult {
        if mask_buf.is_empty() {
            return MaskResult::FullCover;
        }

        if abs_y < self.coords.y1 || abs_y > self.coords.y2 {
            return MaskResult::FullCover;
        }

        let len = mask_buf.len() as i32;
        let start_x = abs_x;
        let end_x = abs_x + len - 1;
        if end_x < self.coords.x1 || start_x > self.coords.x2 {
            return MaskResult::FullCover;
        }

        let slice_start = (self.coords.x1 - abs_x).max(0).min(len);
        let slice_end = (self.coords.x2 - abs_x + 1).max(0).min(len);
        if slice_start >= slice_end {
            return MaskResult::FullCover;
        }

        let opa = self.opacity_for_row(abs_y);
        let slice = &mut mask_buf[slice_start as usize..slice_end as usize];
        for px in slice {
            *px = mask_mix(*px, opa);
        }

        MaskResult::Changed
    }

    fn opacity_for_row(&self, abs_y: i32) -> Opa {
        if abs_y <= self.y_top {
            return self.opa_top;
        }
        if abs_y >= self.y_bottom {
            return self.opa_bottom;
        }

        let opa_diff = self.opa_bottom as i32 - self.opa_top as i32;
        let y_diff = (self.y_bottom - self.y_top + 1).max(1);
        let rel = abs_y - self.y_top;
        let mixed = ((rel as i32 * opa_diff) >> 8) / y_diff;
        let mut result = self.opa_top as i32 + mixed;
        if result < 0 {
            result = 0;
        } else if result > 255 {
            result = 255;
        }
        result as Opa
    }
}
