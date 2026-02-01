//! Luma4 pixel format rasterizer (4-bit grayscale, 2 pixels per byte)

use core::{cmp::min, ptr};

use crate::colors::Color;
use crate::rasterizer::RasterTarget;
use math::udiv255;



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

        // Convert RGB to luma using fast approximation: (R*77 + G*151 + B*28) >> 8
        let luma = color_to_luma4(color);

        unsafe {
            fill_hspan_unchecked(self.buffer.as_mut_ptr(), self.width, y, x_start, actual_len, luma);
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
        let src_luma = color_to_luma4(color);
        let alpha = color.a();

        unsafe {
            blend_solid_hspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                y,
                x_start,
                coverage,
                src_luma,
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

        let luma = color_to_luma4(color);
        
        unsafe {
            fill_vspan_unchecked(self.buffer.as_mut_ptr(), self.width, x, y_start, actual_len, luma);
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
        let src_luma = color_to_luma4(color);
        let alpha = color.a();

        unsafe {
            blend_solid_vspan_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                x,
                y_start,
                coverage,
                src_luma,
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

        let luma = color_to_luma4(color);

        unsafe {
            fill_rect_unchecked(
                self.buffer.as_mut_ptr(),
                self.width,
                x,
                y,
                actual_width,
                actual_height,
                luma,
            );
        }
    }
}

// ============================================================================
// OPTIMIZED CONVERSION & BLENDING PRIMITIVES
// ============================================================================

/// Convert RGBA color to 4-bit luma with fast fixed-point arithmetic
/// Uses BT.601 weights approximated: (77R + 151G + 28B) / 256
#[inline(always)]
fn color_to_luma4(color: Color) -> u8 {
    let r = color.r() as u32;
    let g = color.g() as u32;
    let b = color.b() as u32;
    
    // Fast approximate RGB->Luma conversion
    let luma8 = (r * 77 + g * 151 + b * 28) >> 8;
    
    // Scale to 4-bit (0-15)
    (luma8 >> 4) as u8
}

/// Blend 4-bit luma values with coverage and alpha - OPTIMIZED for minimal branches
#[inline(always)]
fn blend_luma4(dst: u8, src: u8, alpha: u8, coverage: u8) -> u8 {
    // Combine alpha and coverage - branchless when alpha is 255
    let effective_alpha = if alpha == 255 {
        coverage as u32
    } else {
        udiv255((alpha as u32) * (coverage as u32))
    };
    
    // Completely branchless blend using standard formula:
    // result = (src * alpha + dst * (255 - alpha)) / 255
    let inv_alpha = 255 - effective_alpha;
    let blended = udiv255((src as u32) * effective_alpha + (dst as u32) * inv_alpha);
    blended as u8
}

// ============================================================================
// UNSAFE OPTIMIZED SPAN OPERATIONS
// ============================================================================

/// Fill horizontal span - OPTIMIZED with word-level writes where possible
#[inline(always)]
unsafe fn fill_hspan_unchecked(
    buffer: *mut u8,
    width: u16,
    y: u16,
    x_start: u16,
    len: u16,
    luma: u8,
) {
    unsafe {
        // Use linear pixel indexing: pixel_idx = y * width + x
        let start_pixel_idx = (y as usize) * (width as usize) + (x_start as usize);
        let mut byte_idx = start_pixel_idx >> 1;
        let mut pixel_idx = start_pixel_idx;
        let mut remaining = len;
        
        // Duplicate luma to both nibbles for fast fill
        let luma_both = (luma << 4) | luma;
        
        // Handle leading odd pixel (low nibble)
        if (pixel_idx & 1) != 0 && remaining > 0 {
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            ptr::write_volatile(byte_ptr, (current & 0xF0) | luma);
            byte_idx += 1;
            remaining -= 1;
        }
        
        // Fast path: fill pairs of pixels using byte writes
        let pairs = (remaining >> 1) as usize;
        
        if pairs > 0 {
            let byte_ptr = buffer.add(byte_idx);
            // Use ptr::write_bytes for memset-like performance when possible
            // This compiles to efficient word-aligned writes on RISC-V
            if pairs >= 4 {
                ptr::write_bytes(byte_ptr, luma_both, pairs);
            } else {
                // Manual unrolling for small spans
                let mut ptr = byte_ptr;
                for _ in 0..pairs {
                    ptr::write_volatile(ptr, luma_both);
                    ptr = ptr.add(1);
                }
            }
            byte_idx += pairs;
            remaining -= (pairs as u16) << 1;
        }
        
        // Handle trailing odd pixel (high nibble)
        if remaining > 0 {
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            ptr::write_volatile(byte_ptr, (luma << 4) | (current & 0x0F));
        }
    }
}

/// Blend solid color horizontal span with per-pixel coverage - STREAMLINED
#[inline(always)]
unsafe fn blend_solid_hspan_unchecked(
    buffer: *mut u8,
    width: u16,
    y: u16,
    x_start: u16,
    coverage: &[u8],
    src_luma: u8,
    alpha: u8,
) {
    unsafe {
        let start_pixel_idx = (y as usize) * (width as usize) + (x_start as usize);
        let mut pixel_idx = start_pixel_idx;
        let mut cov_ptr = coverage.as_ptr();
        let remaining = coverage.len();
        
        // Optimized: process pairs of pixels when aligned
        let mut i = 0;
        
        // Handle leading odd pixel
        if (pixel_idx & 1) != 0 && i < remaining {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            let cov = ptr::read(cov_ptr);
            let dst_luma = current & 0x0F;
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            ptr::write_volatile(byte_ptr, (current & 0xF0) | blended);
            pixel_idx += 1;
            cov_ptr = cov_ptr.add(1);
            i += 1;
        }
        
        // Process pairs of pixels together (same byte)
        while i + 1 < remaining {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let cov0 = ptr::read(cov_ptr);
            let cov1 = ptr::read(cov_ptr.add(1));
            
            let dst_hi = current >> 4;
            let dst_lo = current & 0x0F;
            
            let blend_hi = blend_luma4(dst_hi, src_luma, alpha, cov0);
            let blend_lo = blend_luma4(dst_lo, src_luma, alpha, cov1);
            
            let new_val = (blend_hi << 4) | blend_lo;
            ptr::write_volatile(byte_ptr, new_val);
            
            pixel_idx += 2;
            cov_ptr = cov_ptr.add(2);
            i += 2;
        }
        
        // Handle trailing pixel
        if i < remaining {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            let cov = ptr::read(cov_ptr);
            let dst_luma = current >> 4;
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            ptr::write_volatile(byte_ptr, (blended << 4) | (current & 0x0F));
        }
    }
}

/// Blend per-pixel colors horizontal span - STREAMLINED
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
        let start_pixel_idx = (y as usize) * (width as usize) + (x_start as usize);
        let mut pixel_idx = start_pixel_idx;
        let mut color_ptr = colors.as_ptr();
        let mut cov_ptr = coverage.as_ptr();
        let len = colors.len();
        
        let mut i = 0;
        
        // Handle leading odd pixel
        if (pixel_idx & 1) != 0 && i < len {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let color = ptr::read(color_ptr);
            let cov = ptr::read(cov_ptr);
            let src_luma = color_to_luma4(color);
            let alpha = color.a();
            
            let dst_luma = current & 0x0F;
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            ptr::write_volatile(byte_ptr, (current & 0xF0) | blended);
            
            pixel_idx += 1;
            color_ptr = color_ptr.add(1);
            cov_ptr = cov_ptr.add(1);
            i += 1;
        }
        
        // Process pairs of pixels together (same byte)
        while i + 1 < len {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let color0 = ptr::read(color_ptr);
            let color1 = ptr::read(color_ptr.add(1));
            let cov0 = ptr::read(cov_ptr);
            let cov1 = ptr::read(cov_ptr.add(1));
            
            let src_luma0 = color_to_luma4(color0);
            let src_luma1 = color_to_luma4(color1);
            let alpha0 = color0.a();
            let alpha1 = color1.a();
            
            let dst_hi = current >> 4;
            let dst_lo = current & 0x0F;
            
            let blend_hi = blend_luma4(dst_hi, src_luma0, alpha0, cov0);
            let blend_lo = blend_luma4(dst_lo, src_luma1, alpha1, cov1);
            
            let new_val = (blend_hi << 4) | blend_lo;
            ptr::write_volatile(byte_ptr, new_val);
            
            pixel_idx += 2;
            color_ptr = color_ptr.add(2);
            cov_ptr = cov_ptr.add(2);
            i += 2;
        }
        
        // Handle trailing pixel
        if i < len {
            let byte_idx = pixel_idx >> 1;
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let color = ptr::read(color_ptr);
            let cov = ptr::read(cov_ptr);
            let src_luma = color_to_luma4(color);
            let alpha = color.a();
            
            let dst_luma = current >> 4;
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            ptr::write_volatile(byte_ptr, (blended << 4) | (current & 0x0F));
        }
    }
}

/// Fill vertical span
#[inline(always)]
unsafe fn fill_vspan_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y_start: u16,
    len: u16,
    luma: u8,
) {
    unsafe {
        // Use linear pixel indexing
        let start_pixel_idx = (y_start as usize) * (width as usize) + (x as usize);
        let mut pixel_idx = start_pixel_idx;
        let pixel_stride = width as usize;
        
        for _ in 0..len {
            let byte_idx = pixel_idx >> 1;
            let is_high_nibble = (pixel_idx & 1) == 0;
            
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let new_val = if is_high_nibble {
                (luma << 4) | (current & 0x0F)
            } else {
                (current & 0xF0) | luma
            };
            
            ptr::write_volatile(byte_ptr, new_val);
            pixel_idx += pixel_stride;
        }
    }
}

/// Blend solid color vertical span - STREAMLINED
#[inline(always)]
unsafe fn blend_solid_vspan_unchecked(
    buffer: *mut u8,
    width: u16,
    x: u16,
    y_start: u16,
    coverage: &[u8],
    src_luma: u8,
    alpha: u8,
) {
    unsafe {
        let start_pixel_idx = (y_start as usize) * (width as usize) + (x as usize);
        let mut pixel_idx = start_pixel_idx;
        let pixel_stride = width as usize;
        let mut cov_ptr = coverage.as_ptr();
        
        for _ in 0..coverage.len() {
            let byte_idx = pixel_idx >> 1;
            let is_high_nibble = (pixel_idx & 1) == 0;
            
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            let cov = ptr::read(cov_ptr);
            
            let dst_luma = if is_high_nibble { current >> 4 } else { current & 0x0F };
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            
            let new_val = if is_high_nibble {
                (blended << 4) | (current & 0x0F)
            } else {
                (current & 0xF0) | blended
            };
            
            ptr::write_volatile(byte_ptr, new_val);
            
            pixel_idx += pixel_stride;
            cov_ptr = cov_ptr.add(1);
        }
    }
}

/// Blend per-pixel colors vertical span - STREAMLINED
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
        let start_pixel_idx = (y_start as usize) * (width as usize) + (x as usize);
        let mut pixel_idx = start_pixel_idx;
        let pixel_stride = width as usize;
        let mut color_ptr = colors.as_ptr();
        let mut cov_ptr = coverage.as_ptr();
        
        for _ in 0..colors.len() {
            let byte_idx = pixel_idx >> 1;
            let is_high_nibble = (pixel_idx & 1) == 0;
            
            let byte_ptr = buffer.add(byte_idx);
            let current = ptr::read_volatile(byte_ptr);
            
            let color = ptr::read(color_ptr);
            let cov = ptr::read(cov_ptr);
            let src_luma = color_to_luma4(color);
            let alpha = color.a();
            
            let dst_luma = if is_high_nibble { current >> 4 } else { current & 0x0F };
            let blended = blend_luma4(dst_luma, src_luma, alpha, cov);
            
            let new_val = if is_high_nibble {
                (blended << 4) | (current & 0x0F)
            } else {
                (current & 0xF0) | blended
            };
            
            ptr::write_volatile(byte_ptr, new_val);
            
            pixel_idx += pixel_stride;
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
    luma: u8,
) {
    unsafe {
        let luma_both = (luma << 4) | luma;
        
        // Special case: full-width aligned rectangles with even width
        if x == 0 && rect_width == width && (width & 1) == 0 {
            let start_pixel_idx = (y as usize) * (width as usize);
            let start_byte_idx = start_pixel_idx >> 1;
            let total_bytes = ((rect_height as usize) * (width as usize)) >> 1;
            let ptr = buffer.add(start_byte_idx);
            ptr::write_bytes(ptr, luma_both, total_bytes);
            return;
        }
        
        // General case: process row by row with optimizations
        for row in 0..rect_height {
            let current_y = y + row;
            fill_hspan_unchecked(buffer, width, current_y, x, rect_width, luma);
        }
    }
}