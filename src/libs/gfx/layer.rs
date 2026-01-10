/// Layer - a rendering surface with coordinate mapping and metadata
/// 
/// Based on LVGL's layer system. A layer represents a rectangular region
/// that can be rendered to, with support for:
/// - Coordinate mapping (logical screen coords to buffer coords)
/// - Clipping
/// - Strip rendering (for memory-efficient partial rendering)
/// - Opacity
/// - Compositing

use super::core::{Color, ColorAlpha, ColorFormat, Opacity, Point, Rect};
use super::draw_target::DrawTarget;

/// A rendering layer with buffer and metadata
pub struct Layer<'a> {
    /// Pixel buffer (owned or borrowed)
    buffer: &'a mut [u8],
    /// Buffer width in pixels
    width: u32,
    /// Buffer height in pixels
    height: u32,
    /// Which screen region the buffer represents
    /// For full-screen: Rect::new(0, 0, width, height)
    /// For strip: Rect::new(0, y_start, width, strip_height)
    buf_area: Rect,
    /// Clipping rectangle (in screen coordinates)
    clip_area: Rect,
    /// Y offset for strip rendering
    /// When rendering strips, this maps buffer[0] to screen line partial_y_offset
    partial_y_offset: i32,
    /// Layer-wide opacity
    opacity: Opacity,
    /// Pixel format
    color_format: ColorFormat,
}

impl<'a> Layer<'a> {
    /// Create a new layer from a buffer
    /// 
    /// # Arguments
    /// * `buffer` - Pixel buffer
    /// * `width` - Buffer width in pixels
    /// * `height` - Buffer height in pixels
    /// * `format` - Pixel format
    pub fn new(buffer: &'a mut [u8], width: u32, height: u32, format: ColorFormat) -> Self {
        let buf_area = Rect::new(0, 0, width, height);
        Self {
            buffer,
            width,
            height,
            buf_area,
            clip_area: buf_area,
            partial_y_offset: 0,
            opacity: Opacity::OPAQUE,
            color_format: format,
        }
    }
    
    /// Create a layer from any DrawTarget
    /// 
    /// This is the primary way to create a layer for rendering.
    /// The layer will render to the entire draw target.
    pub fn from_draw_target(target: &'a mut impl DrawTarget) -> Self {
        let (width, height) = target.dimensions();
        let format = target.color_format();
        let buffer = target.buffer_mut();
        Self::new(buffer, width, height, format)
    }
    
    /// Create a layer for a sub-region of a draw target
    pub fn from_draw_target_area(
        target: &'a mut impl DrawTarget,
        area: Rect,
    ) -> Self {
        let (width, height) = target.dimensions();
        let format = target.color_format();
        let buffer = target.buffer_mut();
        
        // Clip area to target bounds
        let full_rect = Rect::new(0, 0, width, height);
        let clipped_area = area.intersection(full_rect).unwrap_or(full_rect);
        
        let mut layer = Self::new(buffer, width, height, format);
        layer.buf_area = clipped_area;
        layer.clip_area = clipped_area;
        layer
    }
    
    /// Configure for strip rendering
    /// 
    /// Strip rendering is used for memory-efficient rendering of large displays.
    /// Instead of a full framebuffer, only a horizontal strip is rendered at a time.
    /// 
    /// # Arguments
    /// * `screen_y_start` - First screen line this strip represents
    /// * `screen_y_end` - Last screen line (inclusive)
    pub fn set_strip(&mut self, screen_y_start: i32, screen_y_end: i32) {
        self.buf_area = Rect::new(
            0,
            screen_y_start,
            self.width,
            (screen_y_end - screen_y_start + 1) as u32,
        );
        self.clip_area = self.buf_area;
        self.partial_y_offset = screen_y_start;
    }
    
    /// Set clipping rectangle (in screen coordinates)
    pub fn set_clip(&mut self, clip: Rect) {
        // Clip to buf_area
        self.clip_area = clip.intersection(self.buf_area).unwrap_or(self.buf_area);
    }
    
    /// Set layer opacity
    pub fn set_opacity(&mut self, opacity: Opacity) {
        self.opacity = opacity;
    }
    
    /// Get layer width
    pub fn width(&self) -> u32 {
        self.width
    }
    
    /// Get layer height
    pub fn height(&self) -> u32 {
        self.height
    }
    
    /// Get buffer area (screen region this buffer represents)
    pub fn buf_area(&self) -> Rect {
        self.buf_area
    }
    
    /// Get clip area
    pub fn clip_area(&self) -> Rect {
        self.clip_area
    }
    
    /// Get color format
    pub fn color_format(&self) -> ColorFormat {
        self.color_format
    }
    
    /// Get layer opacity
    pub fn opacity(&self) -> Opacity {
        self.opacity
    }
    
    /// Clear layer to a color
    pub fn clear(&mut self, color: Color) {
        // Use blend() for consistency - proper LVGL architecture
        let clear_rect = Rect::new(0, 0, self.width, self.height);
        let color_alpha = ColorAlpha::from_color(color, 255);
        let blend_desc = super::core::blend::BlendDescriptor::solid_fill(
            clear_rect,
            color_alpha,
            super::core::blend::BlendMode::Normal,
        );
        self.blend(&blend_desc);
    }
    
    /// Map screen coordinates to buffer coordinates
    /// Returns None if point is outside buf_area
    pub fn screen_to_buffer(&self, screen_point: Point) -> Option<Point> {
        if !self.buf_area.contains(screen_point) {
            return None;
        }
        
        Some(Point::new(
            screen_point.x - self.buf_area.x,
            screen_point.y - self.buf_area.y,
        ))
    }
    
    /// Check if a screen point is visible (within clip area)
    pub fn is_visible(&self, screen_point: Point) -> bool {
        self.clip_area.contains(screen_point)
    }
    
    /// Check if a screen rectangle intersects the visible area
    pub fn is_rect_visible(&self, screen_rect: Rect) -> bool {
        self.clip_area.intersects(screen_rect)
    }
    
    /// Get pixel index in buffer for screen coordinates
    /// Returns None if coordinates are out of bounds
    pub fn pixel_index(&self, screen_point: Point) -> Option<usize> {
        let buf_point = self.screen_to_buffer(screen_point)?;
        if buf_point.x < 0 || buf_point.y < 0 {
            return None;
        }
        if buf_point.x >= self.width as i32 || buf_point.y >= self.height as i32 {
            return None;
        }
        
        let idx = (buf_point.y as u32 * self.width + buf_point.x as u32) as usize;
        Some(idx * self.color_format.bytes_per_pixel())
    }
    
    /// Get mutable buffer slice
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer
    }
    
    /// Get immutable buffer slice
    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }
}

// Implement DrawTarget for Layer to enable primitives to use blend()
impl<'a> DrawTarget for Layer<'a> {
    fn color_format(&self) -> ColorFormat {
        self.color_format
    }
    
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer
    }
    
    fn buffer(&self) -> &[u8] {
        self.buffer
    }
    
    fn blend(&mut self, desc: &super::core::blend::BlendDescriptor) {
        // Layer doesn't handle dirty regions - that's the responsibility
        // of the underlying DrawingSurface
        // For now, just dispatch to format-specific blending
        
        use super::core::blend::{BlendMode, BlendSource};
        
        // Clip to layer's clip area
        let Some(clipped_rect) = desc.dest_rect.intersection(self.clip_area) else {
            return; // Completely clipped
        };
        
        // Map to buffer coordinates
        use super::core::Point;
        let Some(buf_start) = self.screen_to_buffer(Point::new(clipped_rect.x, clipped_rect.y)) else {
            return; // Outside buffer area
        };
        
        // For now, implement simple solid color fill
        // TODO: Implement full blend functionality with masks, gradients, images
        match &desc.source {
            Some(BlendSource::SolidColor { color }) => {
                self.blend_fill_simple(clipped_rect, *color, desc.opacity);
            }
            _ => {
                // Other blend sources not yet implemented
            }
        }
    }
    
    fn clear(&mut self, color: Color) {
        self.clear(color);
    }
}

impl<'a> Layer<'a> {
    /// Simple solid color fill (internal helper for blend)
    fn blend_fill_simple(&mut self, rect: Rect, color: ColorAlpha, opacity: Opacity) {
        use super::core::{ColorFormat, Point};
        
        // Calculate effective opacity
        let effective_opa = ((color.a as u16 * opacity.value() as u16) / 255) as u8;
        
        match self.color_format {
            ColorFormat::Rgb565 => {
                self.blend_fill_rgb565(rect, color, effective_opa);
            }
            _ => {
                // Other formats not yet implemented
            }
        }
    }
    
    /// RGB565 blend fill
    fn blend_fill_rgb565(&mut self, rect: Rect, color: ColorAlpha, opacity: u8) {
        use super::blend_rgb565;
        use super::core::Point;
        
        // Map to buffer coordinates
        let Some(buf_start) = self.screen_to_buffer(Point::new(rect.x, rect.y)) else {
            return;
        };
        
        if buf_start.x < 0 || buf_start.y < 0 {
            return;
        }
        
        let buf_x = buf_start.x as u32;
        let buf_y = buf_start.y as u32;
        
        // Clamp to buffer bounds
        let max_width = (self.width - buf_x).min(rect.width);
        let max_height = (self.height - buf_y).min(rect.height);
        
        if max_width == 0 || max_height == 0 {
            return;
        }
        
        // Convert to RGB565
        let rgb565 = color.to_color().to_rgb565();
        
        if opacity == 255 {
            // Fast path: opaque
            for y in 0..max_height {
                let row_offset = ((buf_y + y) * self.width + buf_x) as usize * 2;
                for x in 0..max_width {
                    let idx = row_offset + x as usize * 2;
                    if idx + 1 < self.buffer.len() {
                        self.buffer[idx] = (rgb565 >> 8) as u8;
                        self.buffer[idx + 1] = rgb565 as u8;
                    }
                }
            }
        } else if opacity > 0 {
            // Slow path: alpha blending
            for y in 0..max_height {
                let row_offset = ((buf_y + y) * self.width + buf_x) as usize * 2;
                for x in 0..max_width {
                    let idx = row_offset + x as usize * 2;
                    if idx + 1 < self.buffer.len() {
                        let bg = ((self.buffer[idx] as u16) << 8) | (self.buffer[idx + 1] as u16);
                        let blended = blend_rgb565(bg, rgb565, opacity);
                        self.buffer[idx] = (blended >> 8) as u8;
                        self.buffer[idx + 1] = blended as u8;
                    }
                }
            }
        }
    }
}

// Primitive drawing methods will be added via extension traits in the primitives module
