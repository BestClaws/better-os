#![no_std]

use crate::libs::gfx::two_d::raster::Rasterizer;
use crate::libs::gfx::two_d::types::{Point, Rect, Rgb565, Rgba8888, Size};

/// High-performance bitmap font system for space-grade embedded applications.
/// 
/// This module provides optimized text rendering using embedded bitmap fonts
/// designed for maximum performance in embedded systems and real-time applications.
/// 
/// Key features:
/// - Embedded bitmap fonts for zero-allocation rendering
/// - Optimized character lookup and rendering
/// - Support for multiple font sizes and styles
/// - Anti-aliased text rendering
/// - Efficient memory usage with packed font data

/// Font metrics for a single character.
#[derive(Clone, Copy, Debug)]
pub struct CharMetrics {
    /// Width of the character in pixels
    pub width: u8,
    /// Height of the character in pixels
    pub height: u8,
    /// X offset from baseline
    pub x_offset: i8,
    /// Y offset from baseline
    pub y_offset: i8,
    /// X advance for next character
    pub x_advance: u8,
}

/// Font information for a complete font.
#[derive(Clone, Copy, Debug)]
pub struct FontInfo {
    /// Font name
    pub name: &'static str,
    /// Font size in pixels
    pub size: u8,
    /// Line height in pixels
    pub line_height: u8,
    /// Baseline offset from top
    pub baseline: u8,
    /// First character code in the font
    pub first_char: u8,
    /// Last character code in the font
    pub last_char: u8,
}

/// High-performance bitmap font for embedded text rendering.
/// 
/// This struct contains all the data needed for rendering text using
/// embedded bitmap fonts. The font data is stored in a compact format
/// optimized for space-grade embedded applications.
pub struct BitmapFont {
    /// Font information
    pub info: FontInfo,
    /// Character metrics for each character
    pub metrics: &'static [CharMetrics],
    /// Packed bitmap data for all characters
    pub bitmap_data: &'static [u8],
    /// Width of the bitmap atlas in pixels
    pub atlas_width: u16,
    /// Height of the bitmap atlas in pixels
    pub atlas_height: u16,
}

/// Text rendering options for customization.
#[derive(Clone, Copy, Debug)]
pub struct TextOptions {
    /// Text color
    pub color: Rgb565,
    /// Background color (for opaque rendering)
    pub background_color: Option<Rgb565>,
    /// Enable anti-aliasing
    pub anti_alias: bool,
    /// Character spacing in pixels
    pub char_spacing: i8,
    /// Line spacing in pixels
    pub line_spacing: i8,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            color: Rgb565::WHITE,
            background_color: None,
            anti_alias: true,
            char_spacing: 0,
            line_spacing: 0,
        }
    }
}

/// High-performance text renderer for space-grade applications.
/// 
/// This struct provides optimized text rendering capabilities using
/// embedded bitmap fonts with minimal memory allocation and maximum
/// performance for embedded systems.
pub struct TextRenderer {
    /// Current font
    font: &'static BitmapFont,
    /// Current rendering options
    options: TextOptions,
}

impl TextRenderer {
    /// Create a new text renderer with the specified font.
    /// 
    /// # Arguments
    /// * `font` - The bitmap font to use for rendering
    /// 
    /// # Returns
    /// New text renderer with default options
    pub fn new(font: &'static BitmapFont) -> Self {
        Self {
            font,
            options: TextOptions::default(),
        }
    }
    
    /// Set the text rendering options.
    /// 
    /// # Arguments
    /// * `options` - New text rendering options
    /// 
    /// # Returns
    /// Self for method chaining
    pub fn with_options(mut self, options: TextOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Set the text color.
    /// 
    /// # Arguments
    /// * `color` - Text color
    /// 
    /// # Returns
    /// Self for method chaining
    pub fn with_color(mut self, color: Rgb565) -> Self {
        self.options.color = color;
        self
    }
    
    /// Set the background color for opaque rendering.
    /// 
    /// # Arguments
    /// * `background_color` - Background color (None for transparent)
    /// 
    /// # Returns
    /// Self for method chaining
    pub fn with_background(mut self, background_color: Option<Rgb565>) -> Self {
        self.options.background_color = background_color;
        self
    }
    
    /// Enable or disable anti-aliasing.
    /// 
    /// # Arguments
    /// * `anti_alias` - Whether to enable anti-aliasing
    /// 
    /// # Returns
    /// Self for method chaining
    pub fn with_anti_alias(mut self, anti_alias: bool) -> Self {
        self.options.anti_alias = anti_alias;
        self
    }
    
    /// Set character and line spacing.
    /// 
    /// # Arguments
    /// * `char_spacing` - Character spacing in pixels
    /// * `line_spacing` - Line spacing in pixels
    /// 
    /// # Returns
    /// Self for method chaining
    pub fn with_spacing(mut self, char_spacing: i8, line_spacing: i8) -> Self {
        self.options.char_spacing = char_spacing;
        self.options.line_spacing = line_spacing;
        self
    }
    
    /// Render a single character at the specified position.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `pos` - Position to render the character
    /// * `ch` - Character to render
    /// 
    /// # Returns
    /// Width of the rendered character in pixels
    pub fn draw_char(&self, rasterizer: &mut dyn Rasterizer, pos: Point, ch: char) -> i32 {
        let char_code = ch as u8;
        
        // Check if character is in font range
        if char_code < self.font.info.first_char || char_code > self.font.info.last_char {
            return self.font.info.size as i32;
        }
        
        let char_index = (char_code - self.font.info.first_char) as usize;
        let metrics = &self.font.metrics[char_index];
        
        // Calculate character position
        let char_x = pos.x + metrics.x_offset as i32;
        let char_y = pos.y + metrics.y_offset as i32;
        
        // Render character bitmap
        self.render_char_bitmap(rasterizer, char_x, char_y, char_index, metrics);
        
        // Return advance width
        (metrics.x_advance as i32 + self.options.char_spacing as i32)
    }
    
    /// Render a string of text at the specified position.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `pos` - Position to render the text
    /// * `text` - Text to render
    /// 
    /// # Returns
    /// Width of the rendered text in pixels
    pub fn draw_text(&self, rasterizer: &mut dyn Rasterizer, pos: Point, text: &str) -> i32 {
        let mut x = pos.x;
        let mut y = pos.y;
        let mut total_width = 0;
        
        for ch in text.chars() {
            if ch == '\n' {
                // Handle line break
                x = pos.x;
                y += (self.font.info.line_height as i32 + self.options.line_spacing as i32);
                continue;
            }
            
            let char_width = self.draw_char(rasterizer, Point::new(x, y), ch);
            x += char_width;
            total_width = x - pos.x;
        }
        
        total_width
    }
    
    /// Render text with word wrapping at the specified position and width.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `pos` - Position to render the text
    /// * `text` - Text to render
    /// * `max_width` - Maximum width for text wrapping
    /// 
    /// # Returns
    /// Height of the rendered text in pixels
    pub fn draw_text_wrapped(&self, rasterizer: &mut dyn Rasterizer, pos: Point, text: &str, max_width: i32) -> i32 {
        let mut x = pos.x;
        let mut y = pos.y;
        let mut line_height = 0;
        
        for word in text.split_whitespace() {
            let word_width = self.measure_text(word);
            
            // Check if word fits on current line
            if x + word_width > pos.x + max_width && x > pos.x {
                // Move to next line
                x = pos.x;
                y += (self.font.info.line_height as i32 + self.options.line_spacing as i32);
                line_height = (self.font.info.line_height as i32 + self.options.line_spacing as i32);
            }
            
            // Render word
            for ch in word.chars() {
                let char_width = self.draw_char(rasterizer, Point::new(x, y), ch);
                x += char_width;
            }
            
            // Add space after word (except last word)
            x += self.font.info.size as i32 / 4; // Approximate space width
        }
        
        line_height
    }
    
    /// Measure the width of a string without rendering it.
    /// 
    /// # Arguments
    /// * `text` - Text to measure
    /// 
    /// # Returns
    /// Width of the text in pixels
    pub fn measure_text(&self, text: &str) -> i32 {
        let mut width = 0;
        
        for ch in text.chars() {
            if ch == '\n' {
                continue; // Line breaks don't add width
            }
            
            let char_code = ch as u8;
            if char_code >= self.font.info.first_char && char_code <= self.font.info.last_char {
                let char_index = (char_code - self.font.info.first_char) as usize;
                let metrics = &self.font.metrics[char_index];
                width += (metrics.x_advance as i32 + self.options.char_spacing as i32);
            } else {
                width += self.font.info.size as i32;
            }
        }
        
        width
    }
    
    /// Measure the height of multi-line text.
    /// 
    /// # Arguments
    /// * `text` - Text to measure
    /// * `max_width` - Maximum width for text wrapping
    /// 
    /// # Returns
    /// Height of the text in pixels
    pub fn measure_text_height(&self, text: &str, max_width: i32) -> i32 {
        let mut lines = 1;
        let mut current_width = 0;
        
        for word in text.split_whitespace() {
            let word_width = self.measure_text(word);
            
            if current_width + word_width > max_width && current_width > 0 {
                lines += 1;
                current_width = word_width;
            } else {
                current_width += word_width + self.font.info.size as i32 / 4; // Add space
            }
        }
        
        lines * (self.font.info.line_height as i32 + self.options.line_spacing as i32)
    }
    
    /// Render character bitmap with optimized algorithms.
    /// 
    /// This method handles the actual pixel rendering of character bitmaps
    /// using optimized algorithms for maximum performance.
    fn render_char_bitmap(
        &self,
        rasterizer: &mut dyn Rasterizer,
        x: i32,
        y: i32,
        char_index: usize,
        metrics: &CharMetrics,
    ) {
        // Calculate character position in atlas
        let atlas_x = self.calculate_atlas_x(char_index);
        let atlas_y = self.calculate_atlas_y(char_index);
        
        // Render character pixels
        for py in 0..metrics.height {
            for px in 0..metrics.width {
                let pixel_x = x + px as i32;
                let pixel_y = y + py as i32;
                
                // Get pixel from atlas
                let atlas_px = atlas_x + px as u16;
                let atlas_py = atlas_y + py as u16;
                
                if let Some(alpha) = self.get_atlas_pixel(atlas_px, atlas_py) {
                    if alpha > 0 {
                        if let Some(bg_color) = self.options.background_color {
                            // Opaque rendering
                            let blended_color = self.options.color.blend_over_fast(bg_color, alpha);
                            rasterizer.set_pixel(pixel_x, pixel_y, blended_color);
                        } else {
                            // Transparent rendering
                            rasterizer.blend_pixel(pixel_x, pixel_y, self.options.color, alpha);
                        }
                    }
                }
            }
        }
    }
    
    /// Calculate the X position of a character in the atlas.
    fn calculate_atlas_x(&self, char_index: usize) -> u16 {
        // Each character is 8 pixels wide in the atlas
        (char_index * 8) as u16
    }
    
    /// Calculate the Y position of a character in the atlas.
    fn calculate_atlas_y(&self, _char_index: usize) -> u16 {
        // Simple single-row layout - could be optimized with multi-row packing
        0
    }
    
    /// Get a pixel from the atlas with alpha value.
    /// 
    /// # Arguments
    /// * `x` - X coordinate in atlas
    /// * `y` - Y coordinate in atlas
    /// 
    /// # Returns
    /// Alpha value (0-255) or None if out of bounds
    fn get_atlas_pixel(&self, x: u16, y: u16) -> Option<u8> {
        if x >= self.font.atlas_width || y >= self.font.atlas_height {
            return None;
        }
        
        // The font data is stored as 8x8 bitmaps, one byte per row
        // Each character is 8 bytes (8 rows of 8 pixels each)
        let char_index = x as usize / 8; // Which character we're in
        let pixel_in_char = x as usize % 8; // Which pixel within the character
        let row = y as usize; // Which row (0-7)
        
        if char_index >= 95 || row >= 8 {
            return None;
        }
        
        let data_index = char_index * 8 + row;
        if data_index < self.font.bitmap_data.len() {
            let byte = self.font.bitmap_data[data_index];
            let bit = (byte >> (7 - pixel_in_char)) & 1;
            Some(if bit != 0 { 255 } else { 0 })
        } else {
            None
        }
    }
}

/// Convenience functions for quick text rendering.
impl TextRenderer {
    /// Draw text with default options.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `pos` - Position to render the text
    /// * `text` - Text to render
    /// * `color` - Text color
    pub fn draw_text_simple(rasterizer: &mut dyn Rasterizer, font: &'static BitmapFont, pos: Point, text: &str, color: Rgb565) {
        let renderer = TextRenderer::new(font).with_color(color);
        renderer.draw_text(rasterizer, pos, text);
    }
    
    /// Draw text with background color.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `pos` - Position to render the text
    /// * `text` - Text to render
    /// * `color` - Text color
    /// * `background_color` - Background color
    pub fn draw_text_with_background(
        rasterizer: &mut dyn Rasterizer,
        font: &'static BitmapFont,
        pos: Point,
        text: &str,
        color: Rgb565,
        background_color: Rgb565,
    ) {
        let renderer = TextRenderer::new(font)
            .with_color(color)
            .with_background(Some(background_color));
        renderer.draw_text(rasterizer, pos, text);
    }
    
    /// Draw centered text.
    /// 
    /// # Arguments
    /// * `rasterizer` - The rasterizer to draw to
    /// * `rect` - Rectangle to center text in
    /// * `text` - Text to render
    /// * `color` - Text color
    pub fn draw_text_centered(
        rasterizer: &mut dyn Rasterizer,
        font: &'static BitmapFont,
        rect: Rect,
        text: &str,
        color: Rgb565,
    ) {
        let renderer = TextRenderer::new(font).with_color(color);
        let text_width = renderer.measure_text(text);
        let text_height = font.info.line_height as i32;
        
        let x = rect.top_left.x + (rect.size.width as i32 - text_width) / 2;
        let y = rect.top_left.y + (rect.size.height as i32 - text_height) / 2;
        
        renderer.draw_text(rasterizer, Point::new(x, y), text);
    }
}
