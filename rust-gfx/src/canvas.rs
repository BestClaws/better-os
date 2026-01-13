/// Canvas/Framebuffer rendering (internal format converts to ARGB8888 for BMP output)
/// This is the main rendering target matching LVGL's layer system
/// Uses Rgba8888 as the color format, converts to ARGB8888 for BMP
use crate::color::{blend_colors, Rgba8888};
use crate::types::{Area, Opa, OPA_COVER};
use crate::Rasterizer;

#[cfg(feature = "std")]
extern crate alloc;
#[cfg(feature = "std")]
use alloc::vec::Vec;

#[cfg(feature = "std")]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    buffer: Vec<Rgba8888>,
}

#[cfg(feature = "std")]
impl Canvas {
    /// Create a new canvas with transparent background
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            buffer: Vec::from_iter(core::iter::repeat(Rgba8888::TRANSPARENT).take(width * height)),
        }
    }

    /// Clear canvas to a specific color
    pub fn clear(&mut self, color: Rgba8888) {
        self.buffer.fill(color);
    }

    /// Get pixel at coordinates (returns transparent if out of bounds)
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Rgba8888 {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return Rgba8888::TRANSPARENT;
        }
        self.buffer[y as usize * self.width + x as usize]
    }

    /// Set pixel at coordinates (does nothing if out of bounds)
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgba8888) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        self.buffer[y as usize * self.width + x as usize] = color;
    }
    /// Blend pixel at coordinates with opacity
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, opa: Opa) {
        self.blend_pixel_internal(x, y, color, opa);
    }
    /// Blend pixel at coordinates with opacity (internal method)
    #[inline]
    fn blend_pixel_internal(&mut self, x: i32, y: i32, color: Rgba8888, opa: Opa) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        // Note: Don't skip opa==0! LVGL stores color info even for fully transparent pixels

        let idx = y as usize * self.width + x as usize;
        let bg = self.buffer[idx];
        self.buffer[idx] = blend_colors(bg, color, opa);
    }

    /// Fill an area with a solid color and opacity
    pub fn fill_area(&mut self, area: &Area, color: Rgba8888, opa: Opa) {
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
                self.blend_pixel_internal(x, y, color, opa);
            }
        }
    }

    /// Get buffer reference
    pub fn buffer(&self) -> &[Rgba8888] {
        &self.buffer
    }

    /// Get mutable buffer reference
    pub fn buffer_mut(&mut self) -> &mut [Rgba8888] {
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

#[cfg(feature = "std")]
impl Rasterizer for Canvas {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        // Return raw bytes - Argb8888 is 4 bytes per pixel
        unsafe {
            core::slice::from_raw_parts_mut(
                self.buffer.as_mut_ptr() as *mut u8,
                self.buffer.len() * 4,
            )
        }
    }

    fn mark_dirty(&mut self, _min_x: i32, _min_y: i32, _max_x: i32, _max_y: i32) {
        // Canvas doesn't track dirty regions for sprite generation
    }

    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba8888, coverage: u8) {
        self.blend_pixel_internal(x, y, color, coverage);
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgba8888) {
        let area = Area {
            x1: x,
            y1: y,
            x2: x + w - 1,
            y2: y + h - 1,
        };
        self.fill_area(&area, color, OPA_COVER);
    }
}
