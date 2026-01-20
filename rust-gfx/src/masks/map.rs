use crate::types::{Area, Opa};
use super::{mask_mix, MaskResult};

/// Opacity map mask descriptor (port of lv_draw_sw_mask_map_param_t).
#[derive(Clone, Debug)]
pub struct MapMask<'a> {
    coords: Area,
    width: usize,
    map: &'a [Opa],
}

impl<'a> MapMask<'a> {
    /// Create a map mask with an opacity buffer matching the supplied coordinates.
    pub fn new(coords: Area, map: &'a [Opa]) -> Self {
        let width = coords.width().max(0) as usize;
        let height = coords.height().max(0) as usize;
        debug_assert!(width == 0 || map.len() >= width * height);
        Self { coords, width, map }
    }

    /// Apply the map mask to the provided buffer.
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

        if self.width == 0 {
            return MaskResult::FullCover;
        }

        let slice_start = (self.coords.x1 - abs_x).max(0).min(len);
        let slice_end = (self.coords.x2 - abs_x + 1).max(0).min(len);
        if slice_start >= slice_end {
            return MaskResult::FullCover;
        }

        let row = (abs_y - self.coords.y1) as usize;
        let map_row_start = row * self.width;
        let col_start = (abs_x + slice_start - self.coords.x1) as usize;
        let length = (slice_end - slice_start) as usize;
        let map_offset = map_row_start + col_start;
        let map_slice = &self.map[map_offset..map_offset + length];

        for (dst, src) in mask_buf[slice_start as usize..slice_end as usize]
            .iter_mut()
            .zip(map_slice.iter())
        {
            *dst = mask_mix(*dst, *src);
        }

        MaskResult::Changed
    }
}
