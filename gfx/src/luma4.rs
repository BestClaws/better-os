//! Luma4 pixel format rasterizer (4-bit grayscale, 2 pixels per byte)

use core::{cmp::min, ptr};

use crate::colors::Color;
use crate::rasterizer::RasterTarget;
use math::udiv255;

#[inline(always)]
fn color_to_luma(color: Color) -> (u8, u8) {
    let r = color.r() as u32;
    let g = color.g() as u32;
    let b = color.b() as u32;
    let luma = ((r * 77 + g * 150 + b * 29 + 128) >> 8) as u8;
    (luma, color.a())
}

#[inline(always)]
fn luma4_from_luma8(luma: u8) -> u8 {
    // Convert 0..255 to 0..15 with rounding using fast divide by 255.
    let scaled = (luma as u32) * 15 + 128;
    (udiv255(scaled) as u8) & 0x0F
}

#[inline(always)]
fn expand_luma4(nibble: u8) -> u8 {
    nibble * 0x11
}

#[inline(always)]
fn mul_div_255(value: u8, factor: u8) -> u8 {
    let product = (value as u32) * (factor as u32) + 128;
    udiv255(product) as u8
}

#[inline(always)]
unsafe fn load_nibble(ptr_base: *const u8, pixel_index: usize) -> u8 {
    let byte = unsafe { *ptr_base.add(pixel_index >> 1) };
    if (pixel_index & 1) == 0 {
        byte >> 4
    } else {
        byte & 0x0F
    }
}

#[inline(always)]
unsafe fn store_nibble(ptr_base: *mut u8, pixel_index: usize, nibble: u8) {
    let byte_ptr = unsafe { ptr_base.add(pixel_index >> 1) };
    let current = unsafe { *byte_ptr };
    let value = if (pixel_index & 1) == 0 {
        (current & 0x0F) | ((nibble & 0x0F) << 4)
    } else {
        (current & 0xF0) | (nibble & 0x0F)
    };
    unsafe { *byte_ptr = value; }
}

#[inline(always)]
unsafe fn write_solid_contiguous(ptr_base: *mut u8, start_pixel: usize, count: usize, nibble: u8) {
    if count == 0 {
        return;
    }

    let mut remaining = count;
    let mut pixel_index = start_pixel;
    let fill_byte = ((nibble & 0x0F) << 4) | (nibble & 0x0F);

    if (pixel_index & 1) != 0 {
        let byte_ptr = unsafe { ptr_base.add(pixel_index >> 1) };
        let current = unsafe { *byte_ptr };
        unsafe {
            *byte_ptr = (current & 0xF0) | (nibble & 0x0F);
        }
        pixel_index += 1;
        remaining -= 1;
    }

    if remaining >= 2 {
        let full_pairs = remaining >> 1;
        unsafe {
            ptr::write_bytes(ptr_base.add(pixel_index >> 1), fill_byte, full_pairs);
        }
        pixel_index += full_pairs << 1;
        remaining &= 1;
    }

    if remaining != 0 {
        let byte_ptr = unsafe { ptr_base.add(pixel_index >> 1) };
        let current = unsafe { *byte_ptr };
        unsafe {
            *byte_ptr = (current & 0x0F) | ((nibble & 0x0F) << 4);
        }
    }
}

#[inline(always)]
fn row_end(width: u16, x_start: u16, length: usize) -> usize {
    let max_span = (width - x_start) as usize;
    min(max_span, length)
}

/// Luma4 rasterizer that wraps a framebuffer
/// Each byte contains two 4-bit grayscale pixels (high nibble = even pixel, low nibble = odd pixel)
pub struct Luma4Rasterizer<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
}

impl<'a> Luma4Rasterizer<'a> {
    /// Create a new Luma4 rasterizer wrapping a framebuffer
    pub fn new(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self { buffer, width, height }
    }
}

impl<'a> RasterTarget for Luma4Rasterizer<'a> {
    #[inline]
    fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    fn height(&self) -> u16 {
        self.height
    }

    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        if y >= self.height || x_start >= self.width || length == 0 {
            return;
        }

        let span = row_end(self.width, x_start, length as usize);
        if span == 0 {
            return;
        }

        let (src_luma, alpha) = color_to_luma(color);
        if alpha == 0 {
            return;
        }

        let base_pixel = y as usize * self.width as usize + x_start as usize;
        let ptr_base = self.buffer.as_mut_ptr();

        if alpha == 255 {
            unsafe {
                write_solid_contiguous(ptr_base, base_pixel, span, luma4_from_luma8(src_luma));
            }
            return;
        }

        let inv_alpha = 255u8.wrapping_sub(alpha);
        let src_scaled = mul_div_255(src_luma, alpha);

        unsafe {
            let mut pixel_index = base_pixel;
            for _ in 0..span {
                let dest_nibble = load_nibble(ptr_base, pixel_index);
                let dest_luma = expand_luma4(dest_nibble);
                let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                let blended = dest_scaled as u16 + src_scaled as u16;
                let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                let out_nibble = luma4_from_luma8(blended_luma);
                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += 1;
            }
        }
    }

    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        if y >= self.height || x_start >= self.width || coverage.is_empty() {
            return;
        }

        let span = row_end(self.width, x_start, coverage.len());
        if span == 0 {
            return;
        }

        let (src_luma, src_alpha) = color_to_luma(color);
        if src_alpha == 0 {
            return;
        }

        let ptr_base = self.buffer.as_mut_ptr();
        let mut pixel_index = y as usize * self.width as usize + x_start as usize;

        unsafe {
            for &cover in &coverage[..span] {
                let effective_alpha = if cover == 255 {
                    src_alpha
                } else if cover == 0 {
                    pixel_index += 1;
                    continue;
                } else {
                    mul_div_255(src_alpha, cover)
                };

                if effective_alpha == 0 {
                    pixel_index += 1;
                    continue;
                }

                let out_nibble = if effective_alpha == 255 {
                    luma4_from_luma8(src_luma)
                } else {
                    let inv_alpha = 255u8.wrapping_sub(effective_alpha);
                    let src_scaled = mul_div_255(src_luma, effective_alpha);
                    let dest_nibble = load_nibble(ptr_base, pixel_index);
                    let dest_luma = expand_luma4(dest_nibble);
                    let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                    let blended = dest_scaled as u16 + src_scaled as u16;
                    let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                    luma4_from_luma8(blended_luma)
                };

                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += 1;
            }
        }
    }

    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        if y >= self.height || x_start >= self.width || colors.is_empty() || coverage.is_empty() {
            return;
        }

        let count = min(colors.len(), coverage.len());
        let span = row_end(self.width, x_start, count);
        if span == 0 {
            return;
        }

        let ptr_base = self.buffer.as_mut_ptr();
        let mut pixel_index = y as usize * self.width as usize + x_start as usize;

        unsafe {
            for i in 0..span {
                let (src_luma, src_alpha) = color_to_luma(colors[i]);
                if src_alpha == 0 {
                    pixel_index += 1;
                    continue;
                }

                let cover = coverage[i];
                let effective_alpha = if cover == 255 {
                    src_alpha
                } else if cover == 0 {
                    pixel_index += 1;
                    continue;
                } else {
                    mul_div_255(src_alpha, cover)
                };

                let out_nibble = if effective_alpha == 255 {
                    luma4_from_luma8(src_luma)
                } else {
                    let inv_alpha = 255u8.wrapping_sub(effective_alpha);
                    let src_scaled = mul_div_255(src_luma, effective_alpha);
                    let dest_nibble = load_nibble(ptr_base, pixel_index);
                    let dest_luma = expand_luma4(dest_nibble);
                    let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                    let blended = dest_scaled as u16 + src_scaled as u16;
                    let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                    luma4_from_luma8(blended_luma)
                };

                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += 1;
            }
        }
    }

    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        if x >= self.width || y_start >= self.height || length == 0 {
            return;
        }

        let span = min((self.height - y_start) as usize, length as usize);
        if span == 0 {
            return;
        }

        let (src_luma, alpha) = color_to_luma(color);
        if alpha == 0 {
            return;
        }

        let mut pixel_index = y_start as usize * self.width as usize + x as usize;
        let stride = self.width as usize;
        let ptr_base = self.buffer.as_mut_ptr();

        unsafe {
            if alpha == 255 {
                let nibble = luma4_from_luma8(src_luma);
                for _ in 0..span {
                    store_nibble(ptr_base, pixel_index, nibble);
                    pixel_index += stride;
                }
                return;
            }

            let inv_alpha = 255u8.wrapping_sub(alpha);
            let src_scaled = mul_div_255(src_luma, alpha);

            for _ in 0..span {
                let dest_nibble = load_nibble(ptr_base, pixel_index);
                let dest_luma = expand_luma4(dest_nibble);
                let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                let blended = dest_scaled as u16 + src_scaled as u16;
                let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                let out_nibble = luma4_from_luma8(blended_luma);
                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += stride;
            }
        }
    }

    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        if x >= self.width || y_start >= self.height || coverage.is_empty() {
            return;
        }

        let span = min((self.height - y_start) as usize, coverage.len());
        if span == 0 {
            return;
        }

        let (src_luma, src_alpha) = color_to_luma(color);
        if src_alpha == 0 {
            return;
        }

        let mut pixel_index = y_start as usize * self.width as usize + x as usize;
        let stride = self.width as usize;
        let ptr_base = self.buffer.as_mut_ptr();

        unsafe {
            for i in 0..span {
                let cover = coverage[i];
                let effective_alpha = if cover == 255 {
                    src_alpha
                } else if cover == 0 {
                    pixel_index += stride;
                    continue;
                } else {
                    mul_div_255(src_alpha, cover)
                };

                if effective_alpha == 0 {
                    pixel_index += stride;
                    continue;
                }

                let out_nibble = if effective_alpha == 255 {
                    luma4_from_luma8(src_luma)
                } else {
                    let inv_alpha = 255u8.wrapping_sub(effective_alpha);
                    let src_scaled = mul_div_255(src_luma, effective_alpha);
                    let dest_nibble = load_nibble(ptr_base, pixel_index);
                    let dest_luma = expand_luma4(dest_nibble);
                    let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                    let blended = dest_scaled as u16 + src_scaled as u16;
                    let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                    luma4_from_luma8(blended_luma)
                };

                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += stride;
            }
        }
    }

    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        if x >= self.width || y_start >= self.height || colors.is_empty() || coverage.is_empty() {
            return;
        }

        let count = min(colors.len(), coverage.len());
        let span = min((self.height - y_start) as usize, count);
        if span == 0 {
            return;
        }

        let mut pixel_index = y_start as usize * self.width as usize + x as usize;
        let stride = self.width as usize;
        let ptr_base = self.buffer.as_mut_ptr();

        unsafe {
            for i in 0..span {
                let (src_luma, src_alpha) = color_to_luma(colors[i]);
                if src_alpha == 0 {
                    pixel_index += stride;
                    continue;
                }

                let cover = coverage[i];
                let effective_alpha = if cover == 255 {
                    src_alpha
                } else if cover == 0 {
                    pixel_index += stride;
                    continue;
                } else {
                    mul_div_255(src_alpha, cover)
                };

                if effective_alpha == 0 {
                    pixel_index += stride;
                    continue;
                }

                let out_nibble = if effective_alpha == 255 {
                    luma4_from_luma8(src_luma)
                } else {
                    let inv_alpha = 255u8.wrapping_sub(effective_alpha);
                    let src_scaled = mul_div_255(src_luma, effective_alpha);
                    let dest_nibble = load_nibble(ptr_base, pixel_index);
                    let dest_luma = expand_luma4(dest_nibble);
                    let dest_scaled = mul_div_255(dest_luma, inv_alpha);
                    let blended = dest_scaled as u16 + src_scaled as u16;
                    let blended_luma = if blended > 255 { 255 } else { blended as u8 };
                    luma4_from_luma8(blended_luma)
                };

                store_nibble(ptr_base, pixel_index, out_nibble);
                pixel_index += stride;
            }
        }
    }
}
