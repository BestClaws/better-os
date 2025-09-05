use core::marker::PhantomData;
use heapless::Vec;
use crate::libs::gfx::two_d::{Rasterizer, Rgb565, Rect, Point, Size};

/// Space-grade canvas implementation optimized for embedded systems.
/// 
/// This canvas provides high-performance 2D rendering with minimal overhead:
/// - Batched dirty region updates to reduce fragmentation
/// - Optimized pixel operations with minimal branching
/// - Efficient memory layout for cache-friendly access
/// - Reduced per-pixel overhead through operation batching

/// A statically-safe, framebuffer-backed drawing canvas.
///
/// Supports dirty region tracking and pixel-level rendering. Efficient
/// for embedded systems that rely on partial screen updates.
///
/// Supported color format: `Rgb565`.
pub struct Canvas<'a> {
    buf: Option<&'a mut [u8]>,
    width: u32,
    height: u32,
    dirty_regions: Vec<Rect, 8>,
    /// Flag to enable/disable dirty region tracking for performance-critical operations
    dirty_tracking_enabled: bool,
    /// Current batch operation bounds for efficient dirty region updates
    batch_bounds: Option<Rect>,
}

// ===== Canvas Implementation =====
impl<'a> Canvas<'a> {

    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: None,
            width,
            height,
            dirty_regions: {
                let mut v: Vec<Rect, 8> = Vec::new();
                v.push(Rect::new(Point::zero(), Size::new(width, height))).ok();
                v
            },
            dirty_tracking_enabled: true,
            batch_bounds: None,
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

    /// Clears the dirty region, marking the canvas as fully flushed.
    pub fn flush(&mut self) {
        self.dirty_regions.clear();
    }

    /// Returns the current dirty regions slice (may be empty).
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    /// Expands the dirty region to include the given rectangle.
    /// Heavily coalesces regions to minimize fragmentation; if capacity is exceeded,
    /// the entire canvas is marked dirty as a safe fallback.
    fn update_dirty(&mut self, mut new_region: Rect) {
        // Clip to canvas bounds first
        let (x0, y0, x1, y1) = clip_rect(&new_region, self.width, self.height);
        if x0 >= x1 || y0 >= y1 { return; }
        new_region = Rect::new(Point::new(x0 as i32, y0 as i32), Size::new(x1 - x0, y1 - y0));

        // Try to merge with any overlapping or touching regions.
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

        if self.dirty_regions.push(new_region).is_err() {
            // Fallback: if capacity exceeded, mark full-screen dirty
            self.dirty_regions.clear();
            self.dirty_regions.push(Rect::new(Point::zero(), Size::new(self.width, self.height))).ok();
        }
    }
    
    
    
}

impl<'a> Canvas<'a> {
    pub fn resize(&mut self, width: u32, height: u32) {
        let required = (width * height * 2) as usize;
        assert!(self._buf_mut().len() >= required, "Buffer too small for Rgb565");

        self.width = width;
        self.height = height;
    }

    pub(crate) fn set_resources(&mut self, buffer: &'a mut [u8]) {
        let required = (self.width * self.height * 2) as usize;
        assert!(buffer.len() >= required, "Buffer too small for Rgb565");
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
        self.update_dirty(Rect::new(Point::zero(), Size::new(self.width, self.height)));
    }

    /// Marks an arbitrary rectangle as dirty (will be clipped and coalesced).
    pub fn mark_dirty(&mut self, area: Rect) {
        self.update_dirty(area);
    }
    
    /// Enable or disable dirty region tracking for performance-critical operations.
    /// 
    /// When disabled, pixel operations will not update dirty regions, providing
    /// significant performance improvements for bulk operations.
    pub fn set_dirty_tracking(&mut self, enabled: bool) {
        self.dirty_tracking_enabled = enabled;
    }
    
    /// Begin a batch operation for efficient dirty region management.
    /// 
    /// During batch operations, individual pixel updates are accumulated into
    /// a single dirty region, reducing fragmentation and improving performance.
    pub fn begin_batch(&mut self, bounds: Rect) {
        self.batch_bounds = Some(bounds);
    }
    
    /// End the current batch operation and update dirty regions.
    pub fn end_batch(&mut self) {
        if let Some(bounds) = self.batch_bounds.take() {
            self.update_dirty(bounds);
        }
    }
    
    /// Fast pixel set without dirty region tracking.
    /// 
    /// This method provides maximum performance for bulk pixel operations
    /// where dirty region tracking is not needed.
    #[inline(always)]
    pub fn set_pixel_fast(&mut self, x: i32, y: i32, color: Rgb565) {
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        
        let idx = ((x + y * self.width) * 2) as usize;
        let raw = color.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
    }
    
    /// Fast pixel blend without dirty region tracking.
    #[inline(always)]
    pub fn blend_pixel_fast(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        
        let idx = ((x + y * self.width) * 2) as usize;
        let hi = self._buf_mut()[idx] as u16;
        let lo = self._buf_mut()[idx + 1] as u16;
        let bg = Rgb565((hi << 8) | lo);
        let out = color.blend_over(bg, alpha);
        let raw = out.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
    }
    
    /// Optimized horizontal line drawing with minimal overhead.
    pub fn draw_hline_fast(&mut self, x1: i32, x2: i32, y: i32, color: Rgb565) {
        if y < 0 || y >= self.height as i32 { return; }
        let (x_start, x_end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let x_start = x_start.max(0).min(self.width as i32 - 1);
        let x_end = x_end.max(0).min(self.width as i32 - 1);
        
        if x_start > x_end { return; }
        
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        
        let start_idx = ((x_start + y * self.width as i32) * 2) as usize;
        let count = (x_end - x_start + 1) as usize;
        
        for i in 0..count {
            let idx = start_idx + i * 2;
            self._buf_mut()[idx] = hi;
            self._buf_mut()[idx + 1] = lo;
        }
    }
    
    /// Optimized vertical line drawing with minimal overhead.
    pub fn draw_vline_fast(&mut self, x: i32, y1: i32, y2: i32, color: Rgb565) {
        if x < 0 || x >= self.width as i32 { return; }
        let (y_start, y_end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        let y_start = y_start.max(0).min(self.height as i32 - 1);
        let y_end = y_end.max(0).min(self.height as i32 - 1);
        
        if y_start > y_end { return; }
        
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        
        for y in y_start..=y_end {
            let idx = ((x + y * self.width as i32) * 2) as usize;
            self._buf_mut()[idx] = hi;
            self._buf_mut()[idx + 1] = lo;
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
impl Rasterizer for Canvas<'_> {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    
    fn set_pixel(&mut self, x: i32, y: i32, color: Rgb565) {
        // Use fast method if dirty tracking is disabled or we're in batch mode
        if !self.dirty_tracking_enabled || self.batch_bounds.is_some() {
            self.set_pixel_fast(x, y, color);
            return;
        }
        
        // Standard implementation with dirty region tracking
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        let idx = ((x + y * self.width) * 2) as usize;
        let raw = color.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
        self.update_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
    }
    
    fn blend_pixel(&mut self, x: i32, y: i32, color: Rgb565, alpha: u8) {
        // Use fast method if dirty tracking is disabled or we're in batch mode
        if !self.dirty_tracking_enabled || self.batch_bounds.is_some() {
            self.blend_pixel_fast(x, y, color, alpha);
            return;
        }
        
        // Standard implementation with dirty region tracking
        if x < 0 || y < 0 { return; }
        let (x, y) = (x as u32, y as u32);
        if x >= self.width || y >= self.height { return; }
        let idx = ((x + y * self.width) * 2) as usize;
        let hi = self._buf_mut()[idx] as u16;
        let lo = self._buf_mut()[idx + 1] as u16;
        let bg = Rgb565((hi << 8) | lo);
        let out = color.blend_over(bg, alpha);
        let raw = out.into_storage();
        self._buf_mut()[idx] = (raw >> 8) as u8;
        self._buf_mut()[idx + 1] = raw as u8;
        self.update_dirty(Rect::new(Point::new(x as i32, y as i32), Size::new(1, 1)));
    }
}




