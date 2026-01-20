//! Shared helpers for primitive renderers.
//!
//! Many primitives rely on mask-based scanline compositing and dirt
//! tracking. Consolidating the boilerplate here keeps the individual
//! draw routines focused on their geometry logic.

use alloc::vec::Vec;

use crate::masks::{apply_masks, MaskRef, MaskResult};
use crate::types::{Area, Opa, OPA_COVER};

pub mod circle_cache;
pub mod geometry;
pub use circle_cache::CircleCache;
pub use geometry::{clip_to_raster, effective_radius, fill_rect_with_clipping, merge_areas};

/// Convenience wrapper around a reusable mask buffer for scanline
/// rendering. Keeping the allocation outside tight loops avoids
/// repeated Vec creations while still allowing callers to grow the
/// buffer when a wider span is needed.
#[derive(Default)]
pub struct MaskBuffer {
    data: Vec<Opa>,
}

impl MaskBuffer {
    /// Ensure the internal buffer can store at least `width` mask
    /// values, filling the active portion with `OPA_COVER` before
    /// returning a mutable slice.
    pub fn prepare(&mut self, width: usize) -> &mut [Opa] {
        if self.data.len() < width {
            self.data.resize(width, OPA_COVER);
        }
        let active = &mut self.data[..width];
        active.fill(OPA_COVER);
        active
    }
}

/// Apply a set of masks to the provided scanline, returning both the
/// mask result (matching LVGL semantics) and the prepared buffer slice.
///
/// The helper resets the buffer to fully opaque coverage before
/// invoking LVGL's mask combiner.
pub fn apply_scanline_masks<'a>(
    buffer: &'a mut MaskBuffer,
    width: usize,
    x0: i32,
    y: i32,
    masks: &[MaskRef],
) -> (MaskResult, &'a mut [Opa]) {
    let buf = buffer.prepare(width);
    let result = apply_masks(masks, buf, x0, y);
    (result, buf)
}
