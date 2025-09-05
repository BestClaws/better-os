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
    /// * `color` - RGB565 color to set
    fn set_pixel(&mut self, x: i32, y: i32, color: crate::libs::gfx::two_d::types::Rgb565);
    
    /// Blends a pixel with the specified color and alpha value.
    /// 
    /// This method performs alpha blending between the source color and the
    /// existing pixel color, using optimized blending algorithms for maximum
    /// performance while maintaining visual quality.
    /// 
    /// # Arguments
    /// * `x` - X coordinate of the pixel
    /// * `y` - Y coordinate of the pixel
    /// * `color` - RGB565 color to blend with
    /// * `alpha` - Alpha value (0-255, where 255 is fully opaque)
    fn blend_pixel(
        &mut self,
        x: i32,
        y: i32,
        color: crate::libs::gfx::two_d::types::Rgb565,
        alpha: u8,
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
    /// * `color` - RGB565 color to set
    fn set_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: crate::libs::gfx::two_d::types::Rgb565) {
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
    /// * `color` - RGB565 color to blend with
    /// * `alpha` - Alpha value (0-255, where 255 is fully opaque)
    fn blend_pixels_horizontal(&mut self, x: i32, y: i32, width: u32, color: crate::libs::gfx::two_d::types::Rgb565, alpha: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..width {
            self.blend_pixel(x + i as i32, y, color, alpha);
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
    /// * `color` - RGB565 color to set
    fn set_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: crate::libs::gfx::two_d::types::Rgb565) {
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
    /// * `color` - RGB565 color to blend with
    /// * `alpha` - Alpha value (0-255, where 255 is fully opaque)
    fn blend_pixels_vertical(&mut self, x: i32, y: i32, height: u32, color: crate::libs::gfx::two_d::types::Rgb565, alpha: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for i in 0..height {
            self.blend_pixel(x, y + i as i32, color, alpha);
        }
    }
    
    /// Sets a rectangular region of pixels with optimized performance.
    /// 
    /// This method provides an optimized path for setting multiple pixels
    /// in a rectangular region, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `rect` - Rectangle defining the region to fill
    /// * `color` - RGB565 color to set
    fn set_pixels_rect(&mut self, rect: crate::libs::gfx::two_d::types::Rect, color: crate::libs::gfx::two_d::types::Rgb565) {
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
    /// * `color` - RGB565 color to blend with
    /// * `alpha` - Alpha value (0-255, where 255 is fully opaque)
    fn blend_pixels_rect(&mut self, rect: crate::libs::gfx::two_d::types::Rect, color: crate::libs::gfx::two_d::types::Rgb565, alpha: u8) {
        // Default implementation using individual pixel operations
        // Implementations should override this for better performance
        for y in rect.top_left.y..=rect.bottom() {
            for x in rect.top_left.x..=rect.right() {
                self.blend_pixel(x, y, color, alpha);
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
    /// The current RGB565 color at the specified coordinates
    fn get_pixel(&self, x: i32, y: i32) -> crate::libs::gfx::two_d::types::Rgb565 {
        // Default implementation - implementations should override this
        // for better performance and actual pixel reading
        crate::libs::gfx::two_d::types::Rgb565::BLACK
    }
    
    /// Clears the entire drawing surface to the specified color.
    /// 
    /// This method provides an optimized path for clearing the entire
    /// drawing surface, which is common in many rendering operations.
    /// 
    /// # Arguments
    /// * `color` - RGB565 color to clear to
    fn clear(&mut self, color: crate::libs::gfx::two_d::types::Rgb565) {
        let full_rect = crate::libs::gfx::two_d::types::Rect::new(
            crate::libs::gfx::two_d::types::Point::zero(),
            crate::libs::gfx::two_d::types::Size::new(self.width(), self.height())
        );
        self.set_pixels_rect(full_rect, color);
    }
}
