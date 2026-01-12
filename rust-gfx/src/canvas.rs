/// Canvas/Framebuffer for ARGB8888 rendering
/// This is the main rendering target matching LVGL's layer system
use crate::color_argb::{Argb8888, blend_colors};
use crate::types::{Area, Opa, OPA_COVER};

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Argb8888>,
}

impl Canvas {
    /// Create a new canvas with transparent background
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: vec![Argb8888::TRANSPARENT; width * height],
        }
    }

    /// Clear canvas to a specific color
    pub fn clear(&mut self, color: Argb8888) {
        self.buffer.fill(color);
    }

    /// Get pixel at coordinates (returns transparent if out of bounds)
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Argb8888 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return Argb8888::TRANSPARENT;
        }
        self.buffer[y as usize * self.width + x as usize]
    }

    /// Set pixel at coordinates (does nothing if out of bounds)
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Argb8888) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.buffer[y as usize * self.width + x as usize] = color;
    }

    /// Blend pixel at coordinates with opacity
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Argb8888, opa: Opa) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        if opa == 0 {
            return;
        }

        let idx = y as usize * self.width + x as usize;
        let bg = self.buffer[idx];
        self.buffer[idx] = blend_colors(bg, color, opa);
    }

    /// Fill an area with a solid color and opacity
    pub fn fill_area(&mut self, area: &Area, color: Argb8888, opa: Opa) {
        if opa == 0 {
            return;
        }

        let x1 = area.x1.max(0);
        let y1 = area.y1.max(0);
        let x2 = area.x2.min(self.width as i32 - 1);
        let y2 = area.y2.min(self.height as i32 - 1);

        if x1 > x2 || y1 > y2 {
            return;
        }

        for y in y1..=y2 {
            for x in x1..=x2 {
                self.blend_pixel(x, y, color, opa);
            }
        }
    }

    /// Get raw buffer as ARGB8888 u32 values (for BMP output)
    pub fn as_raw_argb(&self) -> &[u32] {
        // SAFETY: Argb8888 is repr(transparent) over u32
        unsafe {
            core::slice::from_raw_parts(
                self.buffer.as_ptr() as *const u32,
                self.buffer.len(),
            )
        }
    }

    /// Get mutable raw buffer
    pub fn as_raw_argb_mut(&mut self) -> &mut [u32] {
        unsafe {
            core::slice::from_raw_parts_mut(
                self.buffer.as_mut_ptr() as *mut u32,
                self.buffer.len(),
            )
        }
    }

    /// Get buffer reference
    pub fn buffer(&self) -> &[Argb8888] {
        &self.buffer
    }

    /// Get mutable buffer reference
    pub fn buffer_mut(&mut self) -> &mut [Argb8888] {
        &mut self.buffer
    }
}

/// Helper to apply opacity to a mask buffer (LVGL-style)
/// Multiplies each value in the mask by opa/255
#[inline]
pub fn apply_opa_to_mask(mask: &mut [u8], opa: Opa) {
    if opa == OPA_COVER {
        return;
    }
    for m in mask.iter_mut() {
        *m = ((*m as u32 * opa as u32) / 255) as u8;
    }
}
