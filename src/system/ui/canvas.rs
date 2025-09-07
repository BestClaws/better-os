use core::marker::PhantomData;
use heapless::Vec;
use crate::libs::gfx::two_d::{Rasterizer, Rgb565, Rect, Point, Size};

/// Space-grade canvas implementation with LVGL-inspired dirty region tracking.
/// 
/// This canvas provides high-performance 2D rendering with built-in dirty region management:
/// - Automatic dirty region tracking for all pixel operations
/// - Intelligent region coalescing to minimize fragmentation
/// - Optimized pixel operations with minimal overhead
/// - Efficient memory layout for cache-friendly access
/// - No performance penalty for dirty tracking - it's built into the design

/// A statically-safe, framebuffer-backed drawing canvas with built-in dirty region tracking.
///
/// This canvas automatically tracks all pixel modifications and maintains an efficient
/// list of dirty regions that need to be redrawn. The dirty region tracking is
/// always enabled and optimized for performance - there's no overhead from enabling/disabling.
///
/// Supported color format: `Rgb565`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceFormat { Rgb565 }

impl SurfaceFormat { pub const fn bytes_per_pixel(&self) -> usize { match self { SurfaceFormat::Rgb565 => 2 } } }

pub struct DrawingSurface<'a> {
    buf: Option<&'a mut [u8]>,
    width: u32,
    height: u32,
    /// Bytes per pixel for the underlying buffer (system-selected). Currently RGB565 => 2.
    pixel_bytes: usize,
    /// Logical pixel format carried by the surface
    pixel_format: SurfaceFormat,
    /// Built-in dirty region tracking - always active and optimized
    dirty_regions: Vec<Rect, 8>,
    /// Current operation bounds for efficient region coalescing
    current_operation_bounds: Option<Rect>,
}

// ===== Canvas Implementation =====
impl<'a> DrawingSurface<'a> {

    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: None,
            width,
            height,
            pixel_bytes: 2,
            pixel_format: SurfaceFormat::Rgb565,
            dirty_regions: {
                let mut v: Vec<Rect, 8> = Vec::new();
                v.push(Rect::new(Point::zero(), Size::new(width, height))).ok();
                v
            },
            current_operation_bounds: None,
        }
    }
    
    pub(crate) fn relinquish(&mut self) {
        self.buf = None;
    }

    fn _buf(&self) -> &[u8] {
        if let Some(buf) = &self.buf {
            // return
            buf
        } else {
            panic!("Buffer not initialized");
        }
    }

    fn _buf_mut(&mut self) -> &mut [u8] {
        if let Some(buf) = &mut self.buf {
            // return
            buf
        } else {
            panic!("Buffer not initialized");
        }
    }
    /// Immutable access to the raw pixel buffer.
    pub fn buffer(&self) -> &[u8] {
        self._buf()
    }

    /// Mutable access to the raw pixel buffer.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self._buf_mut()
    }

    /// Returns the canvas width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the canvas height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Bytes per pixel metadata (currently 2 for RGB565)
    pub fn bytes_per_pixel(&self) -> usize { self.pixel_bytes }

    /// Update pixel format byte size. Must be set before `set_resources` to affect checks.
    pub fn set_pixel_bytes(&mut self, bytes: usize) { self.pixel_bytes = bytes; }

    /// Set logical pixel format (updates bytes per pixel accordingly)
    pub fn set_pixel_format(&mut self, fmt: SurfaceFormat) { self.pixel_format = fmt; self.pixel_bytes = fmt.bytes_per_pixel(); }
    pub fn pixel_format(&self) -> SurfaceFormat { self.pixel_format }

    /// Clears the dirty region, marking the canvas as fully flushed.
    pub fn flush(&mut self) {
        self.dirty_regions.clear();
    }

    /// Returns the current dirty regions slice (may be empty).
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    /// Efficiently marks a region as dirty with intelligent coalescing.
    /// 
    /// This method implements LVGL-style dirty region tracking:
    /// 1. Clips the region to canvas bounds
    /// 2. Coalesces with existing overlapping regions
    /// 3. Falls back to full-screen dirty if too many regions accumulate
    /// 
    /// The algorithm is optimized to minimize the number of dirty regions
    /// while maintaining O(n) complexity for typical use cases.
    fn mark_region_dirty(&mut self, mut new_region: Rect) {
        // Clip to canvas bounds first
        let (x0, y0, x1, y1) = clip_rect(&new_region, self.width, self.height);
        if x0 >= x1 || y0 >= y1 { return; }
        new_region = Rect::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0));

        // If we're in the middle of a batched operation, accumulate the bounds
        if let Some(ref mut bounds) = self.current_operation_bounds {
            *bounds = union_rect(*bounds, new_region);
            return;
        }

        // Try to merge with any overlapping or touching regions
        let mut i = 0;
        while i < self.dirty_regions.len() {
            let current = self.dirty_regions[i];
            if intersects_or_touches(&current, &new_region) {
                // Merge and restart scan to catch transitive merges
                new_region = union_rect(current, new_region);
                self.dirty_regions.swap_remove(i);
                i = 0;
                continue;
            }
            i += 1;
        }

        // Add the merged region
        if self.dirty_regions.push(new_region).is_err() {
            // Fallback: if capacity exceeded, mark full-screen dirty
            // This is the same approach LVGL uses when too many regions accumulate
            self.dirty_regions.clear();
            self.dirty_regions.push(Rect::new(Point::zero(), Size::new(self.width, self.height))).ok();
        }
    }
    
    
    
}

impl<'a> DrawingSurface<'a> {
    pub fn resize(&mut self, width: u32, height: u32) {
        let required = (width as usize) * (height as usize) * self.pixel_bytes;
        assert!(self._buf_mut().len() >= required, "Buffer too small for DrawingSurface");

        self.width = width;
        self.height = height;
    }

    pub(crate) fn set_resources(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width as usize) * (self.height as usize) * self.pixel_bytes;
        assert!(buffer.len() >= required, "Buffer too small for DrawingSurface");
        self.buf = Some(buffer);
    }

    pub fn clear_rgb(&mut self, color: Rgb565) {
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        for chunk in self._buf_mut().chunks_mut(2) {
            if chunk.len() == 2 {
                chunk[0] = hi;
                chunk[1] = lo;
            }
        }
        self.mark_region_dirty(Rect::new(Point::zero(), Size::new(self.width, self.height)));
    }

    /// Marks an arbitrary rectangle as dirty (will be clipped and coalesced).
    /// 
    /// This is the public interface for manual dirty region marking,
    /// similar to LVGL's `lv_inv_area()` function.
    pub fn mark_dirty(&mut self, area: Rect) {
        self.mark_region_dirty(area);
    }
    
    /// Begin a batched drawing operation for efficient dirty region management.
    /// 
    /// During batched operations, individual pixel updates are accumulated into
    /// a single dirty region, reducing fragmentation. This is similar to how
    /// LVGL batches operations during widget rendering.
    /// 
    /// The bounds parameter should encompass the entire area that will be modified
    /// during the batch operation.
    pub fn begin_drawing_batch(&mut self, bounds: Rect) {
        self.current_operation_bounds = Some(bounds);
    }
    
    /// End the current drawing batch and commit the accumulated dirty region.
    /// 
    /// This commits the batched operation's dirty region to the main dirty list,
    /// where it will be coalesced with existing regions.
    pub fn end_drawing_batch(&mut self) {
        if let Some(bounds) = self.current_operation_bounds.take() {
            self.mark_region_dirty(bounds);
        }
    }
    
    /// Ultra-fast pixel set with built-in dirty region tracking.
    /// 
    /// This method is optimized for maximum performance while maintaining automatic
    /// dirty region tracking. The dirty region update is deferred during
    /// batched operations for maximum efficiency.
    /// 
    /// Key optimizations:
    /// - Fast bounds checking with early exit
    /// - Optimized memory access patterns
    /// - Efficient dirty region tracking
    /// - Minimal branching for better performance
    #[inline(always)]
    fn set_pixel_internal(&mut self, x: i32, y: i32, color: Rgb565) {
        // Fast bounds checking with early exit
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        
        // Optimized pixel setting using direct memory access
        let idx = ((x + y * self.width) * 2) as usize;
        let raw = color.into_storage();
        let buf = self._buf_mut();
        buf[idx] = (raw >> 8) as u8;
        buf[idx + 1] = raw as u8;
        
        // Only mark dirty if not in a batched operation
        if self.current_operation_bounds.is_none() {
            self.mark_region_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
        }
    }
    
    /// Ultra-fast pixel blend with built-in dirty region tracking.
    /// 
    /// This method is optimized for maximum performance while maintaining automatic
    /// dirty region tracking. The dirty region update is deferred during
    /// batched operations for maximum efficiency.
    /// 
    /// Key optimizations:
    /// - Fast bounds checking with early exit
    /// - Optimized alpha blending using fast path for common values
    /// - Efficient memory access patterns
    /// - Minimal branching for better performance
    #[inline(always)]
    fn blend_pixel_internal(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        // Fast bounds checking with early exit
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        
        // Optimized pixel blending using direct memory access
        let idx = ((x + y * self.width) * 2) as usize;
        let buf = self._buf_mut();
        let hi = buf[idx] as u16;
        let lo = buf[idx + 1] as u16;
        let bg = Rgb565((hi << 8) | lo);
        let out = color.blend_over_fast(bg, alpha);
        let raw = out.into_storage();
        buf[idx] = (raw >> 8) as u8;
        buf[idx + 1] = raw as u8;
        
        // Only mark dirty if not in a batched operation
        if self.current_operation_bounds.is_none() {
            self.mark_region_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
        }
    }
}




/// Clips the given rectangle to canvas bounds and returns coordinates.
fn clip_rect(area: &Rect, width: u32, height: u32) -> (u32, u32, u32, u32) {
    let x0 = area.top_left.x.max(0) as u32;
    let y0 = area.top_left.y.max(0) as u32;
    let x1 = (area.top_left.x as u32 + area.size.width).min(width);
    let y1 = (area.top_left.y as u32 + area.size.height).min(height);
    (x0, y0, x1, y1)
}

/// Computes the union (bounding box) of two rectangles.
fn union_rect(r1: Rect, r2: Rect) -> Rect {
    let left = r1.top_left.x.min(r2.top_left.x);
    let top = r1.top_left.y.min(r2.top_left.y);

    let right = (r1.top_left.x + r1.size.width as i32).max(r2.top_left.x + r2.size.width as i32);
    let bottom = (r1.top_left.y + r1.size.height as i32).max(r2.top_left.y + r2.size.height as i32);

    Rect::with_corners(Point::new(left, top), Point::new(right - 1, bottom - 1))
}

/// Returns true if two rectangles overlap or touch along edges (useful for coalescing dirty regions).
fn intersects_or_touches(a: &Rect, b: &Rect) -> bool {
    let ax0 = a.top_left.x;
    let ay0 = a.top_left.y;
    let ax1 = a.top_left.x + a.size.width as i32; // exclusive
    let ay1 = a.top_left.y + a.size.height as i32; // exclusive

    let bx0 = b.top_left.x;
    let by0 = b.top_left.y;
    let bx1 = b.top_left.x + b.size.width as i32; // exclusive
    let by1 = b.top_left.y + b.size.height as i32; // exclusive

    // Allow touching by expanding B by 1 pixel in each direction
    let bx0t = bx0 - 1;
    let by0t = by0 - 1;
    let bx1t = bx1 + 1;
    let by1t = by1 + 1;

    !(ax1 <= bx0t || ax0 >= bx1t || ay1 <= by0t || ay0 >= by1t)
}
// ===== Rasterizer implementation =====
impl Rasterizer for DrawingSurface<'_> {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    
    /// Set a pixel with automatic dirty region tracking.
    /// 
    /// This method always tracks dirty regions efficiently. During batched
    /// operations, dirty region updates are deferred for maximum performance.
    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb565) {
        self.set_pixel_internal(x, y, color);
    }
    
    /// Blend a pixel with automatic dirty region tracking.
    /// 
    /// This method always tracks dirty regions efficiently. During batched
    /// operations, dirty region updates are deferred for maximum performance.
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        self.blend_pixel_internal(x, y, color, alpha);
    }
    
    /// Optimized horizontal pixel setting with bulk operations.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a horizontal line, which is common in many rendering operations.
    fn set_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: Rgb565) {
        // Fast bounds checking
        if y < 0 || y >= self.height as i32 { return; }
        if x < 0 || x + width as i32 > self.width as i32 { return; }
        
        let start_x = x.max(0) as u32;
        let end_x = (x + width as i32).min(self.width as i32) as u32;
        let actual_width = end_x - start_x;
        
        if actual_width == 0 { return; }
        
        // Use optimized bulk memory operations
        let y_offset = y as u32 * self.width;
        let start_idx = ((start_x + y_offset) * 2) as usize;
        let raw = color.into_storage();
        let hi_byte = (raw >> 8) as u8;
        let lo_byte = raw as u8;
        
        let buf = self._buf_mut();
        for i in 0..actual_width {
            let idx = start_idx + (i * 2) as usize;
            buf[idx] = hi_byte;
            buf[idx + 1] = lo_byte;
        }
        
        // Mark region dirty
        if self.current_operation_bounds.is_none() {
            self.mark_region_dirty(Rect::new(
                Point::new(start_x as i32, y),
                Size::new(actual_width, 1)
            ));
        }
    }
    
    /// Optimized vertical pixel setting with bulk operations.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a vertical line, which is common in many rendering operations.
    fn set_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: Rgb565) {
        // Fast bounds checking
        if x < 0 || x >= self.width as i32 { return; }
        if y < 0 || y + height as i32 > self.height as i32 { return; }
        
        let start_y = y.max(0) as u32;
        let end_y = (y + height as i32).min(self.height as i32) as u32;
        let actual_height = end_y - start_y;
        
        if actual_height == 0 { return; }
        
        // Use optimized bulk memory operations
        let raw = color.into_storage();
        let hi_byte = (raw >> 8) as u8;
        let lo_byte = raw as u8;
        
        let width = self.width;
        let buf = self._buf_mut();
        for i in 0..actual_height {
            let idx = ((x as u32 + (start_y + i) * width) * 2) as usize;
            buf[idx] = hi_byte;
            buf[idx + 1] = lo_byte;
        }
        
        // Mark region dirty
        if self.current_operation_bounds.is_none() {
            self.mark_region_dirty(Rect::new(
                Point::new(x, start_y as i32),
                Size::new(1, actual_height)
            ));
        }
    }
    
    /// Optimized rectangular pixel setting with bulk operations.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a rectangular region, which is common in many rendering operations.
    fn set_pixels_rect(&mut self, rect: Rect, color: Rgb565) {
        // Fast bounds checking
        if rect.size.width == 0 || rect.size.height == 0 { return; }
        
        let clip = Rect::new(Point::zero(), Size::new(self.width, self.height));
        let Some(clipped_rect) = rect.intersection(&clip) else { return; };
        
        // Use optimized bulk memory operations
        let raw = color.into_storage();
        let hi_byte = (raw >> 8) as u8;
        let lo_byte = raw as u8;
        
        let width = self.width;
        let buf = self._buf_mut();
        for y in clipped_rect.top_left.y..=clipped_rect.bottom() {
            let y_offset = y as u32 * width;
            for x in clipped_rect.top_left.x..=clipped_rect.right() {
                let idx = ((x as u32 + y_offset) * 2) as usize;
                buf[idx] = hi_byte;
                buf[idx + 1] = lo_byte;
            }
        }
        
        // Mark region dirty
        if self.current_operation_bounds.is_none() {
            self.mark_region_dirty(clipped_rect);
        }
    }
    
    /// Get pixel color for read-back operations.
    /// 
    /// This method provides pixel read-back functionality for advanced
    /// blending algorithms and effects.
    fn get_pixel(&self, x: i32, y: i32) -> Rgb565 {
        // Fast bounds checking
        if x < 0 || y < 0 { return Rgb565::BLACK; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return Rgb565::BLACK; }
        
        // Read pixel data
        let idx = ((x + y * self.width) * 2) as usize;
        let buf = self._buf();
        let hi = buf[idx] as u16;
        let lo = buf[idx + 1] as u16;
        Rgb565((hi << 8) | lo)
    }
}




