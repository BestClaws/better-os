//! RGB565 pixel format rasterizer (16-bit color, 5-6-5 bit packing)

use core::{cmp::min, ptr};

use crate::colors::Color;
use crate::rasterizer::RasterTarget;
use math::udiv255;

/// RGB565 rasterizer that wraps a framebuffer
/// Each pixel is 16 bits: RRRRRGGG GGGBBBBB
pub struct Rgb565Rasterizer<'a> {
    buffer: &'a mut [u8],
    width: u16,
    height: u16,
}

impl<'a> Rgb565Rasterizer<'a> {
    /// Create a new RGB565 rasterizer wrapping a framebuffer
    /// Buffer must be at least width * height * 2 bytes
    pub fn new(buffer: &'a mut [u8], width: u16, height: u16) -> Self {
        Self { buffer, width, height }
    }
}

impl<'a> RasterTarget for Rgb565Rasterizer<'a> {
    #[inline]
    fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    fn height(&self) -> u16 {
        self.height
    }

    #[inline(always)]
    fn fill_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, length: u16) {
        if length == 0 || y >= self.height {
            return;
        }

        let x_end = min(x_start.saturating_add(length), self.width);
        let actual_len = x_end.saturating_sub(x_start);
        if actual_len == 0 {
            return;
        }

        let rgb565 = color_to_rgb565(color);

        unsafe {
            fill_hspan_unchecked(self.buffer.as_mut_ptr(), self.width, y, x_start, actual_len, rgb565);
        }
    }

    #[inline(always)]
    fn blend_solid_hspan(&mut self, y: u16, x_start: u16, color: Color, coverage: &[u8]) {
        if coverage.is_empty() || y >= self.height {
            return;
        }

        let x_end = min(x_start.saturating_add(coverage.len() as u16), self.width);
        let actual_len = (x_end.saturating_sub(x_start)) as usize;
        if actual_len == 0 {
            return;
        }

        let coverage = &coverage[..min(coverage.len(), actual_len)];
        let src_rgb565 = color_to_rgb565(color);
        let alpha = color.a();

        unsafe {
            blend_solid_hspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                y,
                x_start,
                coverage,
                src_rgb565,
                alpha,
            );
        }
    }

    #[inline(always)]
    fn blend_color_hspan(&mut self, y: u16, x_start: u16, colors: &[Color], coverage: &[u8]) {
        if colors.is_empty() || coverage.is_empty() || y >= self.height {
            return;
        }

        let len = min(colors.len(), coverage.len());
        let x_end = min(x_start.saturating_add(len as u16), self.width);
        let actual_len = (x_end.saturating_sub(x_start)) as usize;
        if actual_len == 0 {
            return;
        }

        let len = min(len, actual_len);

        unsafe {
            blend_color_hspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                y,
                x_start,
                &colors[..len],
                &coverage[..len],
            );
        }
    }

    #[inline(always)]
    fn fill_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, length: u16) {
        if length == 0 || x >= self.width {
            return;
        }

        let y_end = min(y_start.saturating_add(length), self.height);
        let actual_len = y_end.saturating_sub(y_start);
        if actual_len == 0 {
            return;
        }

        let rgb565 = color_to_rgb565(color);

        unsafe {
            fill_vspan_unchecked(self.buffer.as_mut_ptr(), self.width, x, y_start, actual_len, rgb565);
        }
    }

    #[inline(always)]
    fn blend_solid_vspan(&mut self, x: u16, y_start: u16, color: Color, coverage: &[u8]) {
        if coverage.is_empty() || x >= self.width {
            return;
        }

        let y_end = min(y_start.saturating_add(coverage.len() as u16), self.height);
        let actual_len = (y_end.saturating_sub(y_start)) as usize;
        if actual_len == 0 {
            return;
        }

        let coverage = &coverage[..min(coverage.len(), actual_len)];
        let src_rgb565 = color_to_rgb565(color);
        let alpha = color.a();

        unsafe {
            blend_solid_vspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                x,
                y_start,
                coverage,
                src_rgb565,
                alpha,
            );
        }
    }

    #[inline(always)]
    fn blend_color_vspan(&mut self, x: u16, y_start: u16, colors: &[Color], coverage: &[u8]) {
        if colors.is_empty() || coverage.is_empty() || x >= self.width {
            return;
        }

        let len = min(colors.len(), coverage.len());
        let y_end = min(y_start.saturating_add(len as u16), self.height);
        let actual_len = (y_end.saturating_sub(y_start)) as usize;
        if actual_len == 0 {
            return;
        }

        let len = min(len, actual_len);

        unsafe {
            blend_color_vspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                x,
                y_start,
                &colors[..len],
                &coverage[..len],
            );
        }
    }

    #[inline(always)]
    fn fill_solid_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        if width == 0 || height == 0 {
            return;
        }

        let x_end = min(x.saturating_add(width), self.width);
        let y_end = min(y.saturating_add(height), self.height);
        let actual_width = x_end.saturating_sub(x);
        let actual_height = y_end.saturating_sub(y);

        if actual_width == 0 || actual_height == 0 {
            return;
        }

        let rgb565 = color_to_rgb565(color);

        unsafe {
            fill_rect_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                x,
                y,
                actual_width,
                actual_height,
                rgb565,
            );
        }
    }
}

// ============================================================================
// OPTIMIZED CONVERSION & BLENDING PRIMITIVES
// ============================================================================

/// Convert RGBA color to RGB565 format (16-bit: RRRRRGGGGGGBBBBB)
#[inline(always)]
fn color_to_rgb565(color: Color) -> u16 {
    let r = (color.r() as u16) >> 3; // 8-bit to 5-bit
    let g = (color.g() as u16) >> 2; // 8-bit to 6-bit
    let b = (color.b() as u16) >> 3; // 8-bit to 5-bit
    (r << 11) | (g << 5) | b
}

/// Extract RGB565 components back to 8-bit (with expansion)
#[inline(always)]
fn rgb565_to_rgb888(rgb565: u16) -> (u8, u8, u8) {
    let r5 = ((rgb565 >> 11) & 0x1F) as u8;
    let g6 = ((rgb565 >> 5) & 0x3F) as u8;
    let b5 = (rgb565 & 0x1F) as u8;

    // Expand to 8-bit: replicate high bits to low bits for better quality
    let r = (r5 << 3) | (r5 >> 2);
    let g = (g6 << 2) | (g6 >> 4);
    let b = (b5 << 3) | (b5 >> 2);

    (r, g, b)
}

/// Blend RGB565 pixels with coverage and alpha - OPTIMIZED
#[inline(always)]
fn blend_rgb565(dst: u16, src: u16, alpha: u8, coverage: u8) -> u16 {
    // Combine alpha and coverage
    let effective_alpha = if alpha == 255 {
        coverage as u32
    } else {
        udiv255((alpha as u32) * (coverage as u32))
    };

    let inv_alpha = 255 - effective_alpha;

    // Blend in RGB565 space using bit manipulation for speed
    // Extract components
    let src_r = (src >> 11) & 0x1F;
    let src_g = (src >> 5) & 0x3F;
    let src_b = src & 0x1F;

    let dst_r = (dst >> 11) & 0x1F;
    let dst_g = (dst >> 5) & 0x3F;
    let dst_b = dst & 0x1F;

    // Blend each component
    let out_r = udiv255((src_r as u32) * effective_alpha + (dst_r as u32) * inv_alpha);
    let out_g = udiv255((src_g as u32) * effective_alpha + (dst_g as u32) * inv_alpha);
    let out_b = udiv255((src_b as u32) * effective_alpha + (dst_b as u32) * inv_alpha);

    // Pack back to RGB565
    ((out_r as u16) << 11) | ((out_g as u16) << 5) | (out_b as u16)
}

// ============================================================================
// UNSAFE OPTIMIZED SPAN OPERATIONS
// ============================================================================

/// Fill horizontal span - OPTIMIZED
#[inline(always)]
unsafe fn fill_hspan_unchecked(
    buffer: *mut u8,
    width: u16,
    y: u16,
    x_start: u16,
    len: u16,
    rgb565: u16,
) {
    unsafe {
        let row_offset = (y as usize) * (width as usize);
        let pixel_offset = row_offset + (x_start as usize);
        let byte_ptr = buffer.add(pixel_offset * 2);
        let pixel_ptr = byte_ptr as *mut u16;

        for i in 0..(len as usize) {
            ptr::write_volatile(pixel_ptr.add(i), rgb565);
        }
    }
}

/// Blend solid color horizontal span - OPTIMIZED
#[inline(always)]
unsafe fn blend_solid_hspan_unchecked(
    buffer: *mut u8,
    width: u16,
    y: u16,
    x_start: u16,
    coverage: &[u8],
    src_rgb565: u16,
    alpha: u8,
) {
    unsafe {
        let row_offset = (y as usize) * (width as usize);
        let pixel_offset = row_offset + (x_start as usize);
        let byte_ptr = buffer.add(pixel_offset * 2);
        let mut pixel_ptr = byte_ptr as *mut u16;
        let mut cov_ptr = coverage.as_ptr();

        for _ in 0..coverage.len() {
            let dst = ptr::read_volatile(pixel_ptr);
            let cov = ptr::read(cov_ptr);
            let blended = blend_rgb565(dst, src_rgb565, alpha, cov);
            ptr::write_volatile(pixel_ptr, blended);

            pixel_ptr = pixel_ptr.add(1);
            cov_ptr = cov_ptr.add(1);
        }
    }
}

/// Blend per-pixel colors horizontal span - OPTIMIZED
#[inline(always)]
unsafe fn blend_color_hspan_unchecked(
    buffer: *mut u8,
    width: u16,
    y: u16,
    x_start: u16,
    colors: &[Color],
    coverage: &[u8],
) {
    unsafe {
        let row_offset = (y as usize) * (width as usize);
        let pixel_offset = row_offset + (x_start as usize);
        let byte_ptr = buffer.add(pixel_offset * 2);
        let mut pixel_ptr = byte_ptr as *mut u16;
        let mut color_ptr = colors.as_ptr();
        let mut cov_ptr = coverage.as_ptr();

        for _ in 0..colors.len() {
            let color = ptr::read(color_ptr);
            let cov = ptr::read(cov_ptr);
            let src_rgb565 = color_to_rgb565(color);
            let alpha = color.a();

            let dst = ptr::read_volatile(pixel_ptr);
            let blended = blend_rgb565(dst, src_rgb565, alpha, cov);
            ptr::write_volatile(pixel_ptr, blended);

            pixel_ptr = pixel_ptr.add(1);
            color_ptr = color_ptr.add(1);
            cov_ptr = cov_ptr.add(1);
        }
    }
}

/// Fill vertical span - OPTIMIZED
#[inline(always)]
unsafe fn fill_vspan_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y_start: u16,
    len: u16,
    rgb565: u16,
) {
    unsafe {
        let start_offset = (y_start as usize) * (width as usize) + (x as usize);
        let byte_ptr = buffer.add(start_offset * 2);
        let mut pixel_ptr = byte_ptr as *mut u16;
        let row_stride = width as usize;

        for _ in 0..len {
            ptr::write_volatile(pixel_ptr, rgb565);
            pixel_ptr = pixel_ptr.add(row_stride);
        }
    }
}

/// Blend solid color vertical span - OPTIMIZED
#[inline(always)]
unsafe fn blend_solid_vspan_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y_start: u16,
    coverage: &[u8],
    src_rgb565: u16,
    alpha: u8,
) {
    unsafe {
        let start_offset = (y_start as usize) * (width as usize) + (x as usize);
        let byte_ptr = buffer.add(start_offset * 2);
        let mut pixel_ptr = byte_ptr as *mut u16;
        let row_stride = width as usize;
        let mut cov_ptr = coverage.as_ptr();

        for _ in 0..coverage.len() {
            let dst = ptr::read_volatile(pixel_ptr);
            let cov = ptr::read(cov_ptr);
            let blended = blend_rgb565(dst, src_rgb565, alpha, cov);
            ptr::write_volatile(pixel_ptr, blended);

            pixel_ptr = pixel_ptr.add(row_stride);
            cov_ptr = cov_ptr.add(1);
        }
    }
}

/// Blend per-pixel colors vertical span - OPTIMIZED
#[inline(always)]
unsafe fn blend_color_vspan_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y_start: u16,
    colors: &[Color],
    coverage: &[u8],
) {
    unsafe {
        let start_offset = (y_start as usize) * (width as usize) + (x as usize);
        let byte_ptr = buffer.add(start_offset * 2);
        let mut pixel_ptr = byte_ptr as *mut u16;
        let row_stride = width as usize;
        let mut color_ptr = colors.as_ptr();
        let mut cov_ptr = coverage.as_ptr();

        for _ in 0..colors.len() {
            let color = ptr::read(color_ptr);
            let cov = ptr::read(cov_ptr);
            let src_rgb565 = color_to_rgb565(color);
            let alpha = color.a();

            let dst = ptr::read_volatile(pixel_ptr);
            let blended = blend_rgb565(dst, src_rgb565, alpha, cov);
            ptr::write_volatile(pixel_ptr, blended);

            pixel_ptr = pixel_ptr.add(row_stride);
            color_ptr = color_ptr.add(1);
            cov_ptr = cov_ptr.add(1);
        }
    }
}

/// Fill rectangle - highly optimized for RISC-V
#[inline(always)]
unsafe fn fill_rect_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y: u16,
    rect_width: u16,
    rect_height: u16,
    rgb565: u16,
) {
    unsafe {
        // Special case: full-width rectangles can use optimized row fills
        if x == 0 && rect_width == width {
            let start_offset = (y as usize) * (width as usize);
            let total_pixels = (rect_height as usize) * (width as usize);
            let byte_ptr = buffer.add(start_offset * 2);
            let mut ptr = byte_ptr as *mut u16;

            for _ in 0..total_pixels {
                ptr::write_volatile(ptr, rgb565);
                ptr = ptr.add(1);
            }
            return;
        }

        // General case: process row by row
        for row in 0..rect_height {
            let current_y = y + row;
            fill_hspan_unchecked(buffer, width, current_y, x, rect_width, rgb565);
        }
    }
}
