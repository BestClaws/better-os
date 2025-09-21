/// High-performance rasterizer trait for space-grade 2D graphics rendering.
/// 
/// This trait defines the core interface for pixel-level graphics operations
/// with optimizations for embedded systems and real-time rendering applications.
/// 
/// Key design principles:
/// - Minimal overhead for pixel operations
/// - Support for both direct pixel setting and alpha blending
/// - Efficient memory access patterns
/// - Cache-friendly data layouts
/// - SIMD-friendly operation batching
pub trait Rasterizer {
    /// Returns the width of the rasterizer's drawing surface in pixels.
    fn width(&self) -> u32;
    
    /// Returns the height of the rasterizer's drawing surface in pixels.
    fn height(&self) -> u32;
    
    /// Sets a pixel to the specified color with maximum performance.
    /// 
    /// This method should be optimized for the fastest possible pixel setting
    /// operation, as it's called frequently during rendering operations.
    /// 
    /// # Arguments
    /// * `x` - X coordinate of the pixel
    /// * `y` - Y coordinate of the pixel
    /// * `color` - RGBA8888 color to set (alpha is applied as overwrite semantics)
    fn set_pixel(&mut self, x: i32, y: i32, color: super::types::Rgba8888);
    
    /// Blends a pixel with the specified color and alpha value.
    /// 
    /// This method performs alpha blending between the source color and the
    /// existing pixel color, using optimized blending algorithms for maximum
    /// performance while maintaining visual quality.
    /// 
    /// # Arguments
    /// * `x` - X coordinate of the pixel
    /// * `y` - Y coordinate of the pixel
    /// * `color` - RGBA8888 color to blend with
    /// * `coverage` - Additional coverage (0-255). Implementations should multiply
    ///                the color alpha by coverage/255 before blending over dest.
    fn blend_pixel(
        &mut self,
        x: i32,
        y: i32,
        color: super::types::Rgba8888,
        coverage: u8,
    );
    
    /// Sets multiple pixels in a horizontal line with optimized performance.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a horizontal line, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `x` - Starting X coordinate
    /// * `y` - Y coordinate
    /// * `width` - Number of pixels to set
    /// * `color` - RGBA8888 color to set
    fn set_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: super::types::Rgba8888) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..width {
            self.set_pixel(x + i as i32, y, color);
        }
    }
    
    /// Blends multiple pixels in a horizontal line with optimized performance.
    /// 
    /// This method provides an optimized path for blending multiple pixels
    /// in a horizontal line, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `x` - Starting X coordinate
    /// * `y` - Y coordinate
    /// * `width` - Number of pixels to blend
    /// * `color` - RGBA8888 color to blend with
    /// * `coverage` - Additional coverage (0-255)
    fn blend_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: super::types::Rgba8888, coverage: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..width {
            self.blend_pixel(x + i as i32, y, color, coverage);
        }
    }
    
    /// Sets multiple pixels in a vertical line with optimized performance.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a vertical line, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `x` - X coordinate
    /// * `y` - Starting Y coordinate
    /// * `height` - Number of pixels to set
    /// * `color` - RGBA8888 color to set
    fn set_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: super::types::Rgba8888) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..height {
            self.set_pixel(x, y + i as i32, color);
        }
    }
    
    /// Blends multiple pixels in a vertical line with optimized performance.
    /// 
    /// This method provides an optimized path for blending multiple pixels
    /// in a vertical line, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `x` - X coordinate
    /// * `y` - Starting Y coordinate
    /// * `height` - Number of pixels to blend
    /// * `color` - RGBA8888 color to blend with
    /// * `coverage` - Additional coverage (0-255)
    fn blend_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: super::types::Rgba8888, coverage: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..height {
            self.blend_pixel(x, y + i as i32, color, coverage);
        }
    }
    
    /// Sets a rectangular region of pixels with optimized performance.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a rectangular region, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `rect` - Rectangle defining the region to fill
    /// * `color` - RGBA8888 color to set
    fn set_pixels_rect(&mut self, rect: super::types::Rect, color: super::types::Rgba8888) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for y in rect.top_left.y..=rect.bottom() {
            for x in rect.top_left.x..=rect.right() {
                self.set_pixel(x, y, color);
            }
        }
    }
    
    /// Blends a rectangular region of pixels with optimized performance.
    /// 
    /// This method provides an optimized path for blending multiple pixels
    /// in a rectangular region, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `rect` - Rectangle defining the region to fill
    /// * `color` - RGBA8888 color to blend with
    /// * `coverage` - Additional coverage (0-255)
    fn blend_pixels_rect(&mut self, rect: super::types::Rect, color: super::types::Rgba8888, coverage: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for y in rect.top_left.y..=rect.bottom() {
            for x in rect.top_left.x..=rect.right() {
                self.blend_pixel(x, y, color, coverage);
            }
        }
    }
    
    /// Gets the current pixel color at the specified coordinates.
    /// 
    /// This method is useful for read-back operations and advanced blending
    /// algorithms that need to sample the existing pixel data.
    /// 
    /// # Arguments
    /// * `x` - X coordinate of the pixel
    /// * `y` - Y coordinate of the pixel
    /// 
    /// # Returns
    /// The current RGBA8888 color at the specified coordinates
    fn get_pixel(&self, x: i32, y: i32) -> super::types::Rgba8888 {
        // Default implementation - implementations should override this
        // for better performance and actual pixel reading
        super::types::Rgba8888 { r: 0, g: 0, b: 0, a: 255 }
    }
    
    /// Clears the entire drawing surface to the specified color.
    /// 
    /// This method provides an optimized path for clearing the entire
    /// drawing surface, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `color` - RGBA8888 color to clear to
    fn clear(&mut self, color: super::types::Rgba8888) {
        let full_rect = super::types::Rect::new(
            super::types::Point::zero(),
            super::types::Size::new(self.width(), self.height())
        );
        self.set_pixels_rect(full_rect, color);
    }
}
