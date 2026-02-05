//! Font rendering primitives with builder pattern API

use crate::colors::Color;
use crate::rasterizer::RasterTarget;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use swash::scale::image::Content;
use swash::{FontRef, GlyphId, scale::Render, scale::ScaleContext, scale::Source};

/// Cached glyph data
struct CachedGlyph {
    /// Pre-rendered glyph image data (alpha mask)
    data: Vec<u8>,
    /// Width of the glyph image
    width: usize,
    /// Height of the glyph image
    height: usize,
    /// Left bearing (offset from cursor)
    left: i32,
    /// Top bearing (offset from baseline)
    top: i32,
    /// Horizontal advance (how much to move cursor)
    advance: i32,
    /// Last access timestamp for LRU eviction
    last_access: u32,
}

/// Font renderer with LRU-cached glyphs and kerning support
pub struct Font {
    glyphs: BTreeMap<char, CachedGlyph>,
    font_data: &'static [u8],
    font_size: f32,
    baseline: i32,
    hint: bool,
    /// Current cache size in bytes
    cache_size: usize,
    /// Maximum cache size (4KB)
    max_cache_size: usize,
    /// Timestamp counter for LRU
    access_counter: u32,
}


impl Font {
    /// Create a new font builder
    pub fn builder() -> FontBuilder {
        FontBuilder::new()
    }

    /// Get or cache a glyph on-demand with LRU eviction
    fn get_or_cache_glyph(&mut self, ch: char) -> Option<&CachedGlyph> {
        // Update timestamp and return if already cached
        if let Some(glyph) = self.glyphs.get_mut(&ch) {
            self.access_counter = self.access_counter.wrapping_add(1);
            glyph.last_access = self.access_counter;
            return self.glyphs.get(&ch);
        }

        // Need to render glyph
        let font_ref = FontRef::from_index(self.font_data, 0).expect("Failed to load font");
        let mut scale_context = ScaleContext::new();
        let mut scaler = scale_context
            .builder(font_ref)
            .size(self.font_size)
            .hint(self.hint)
            .build();

        let charmap = font_ref.charmap();
        let glyph_id = charmap.map(ch);

        let img = Render::new(&[Source::Outline]).render(&mut scaler, glyph_id)?;
        if let Content::Mask = img.content {
            let glyph_size = img.data.len() + core::mem::size_of::<CachedGlyph>();
            
            // LRU eviction if cache is full
            while self.cache_size + glyph_size > self.max_cache_size && !self.glyphs.is_empty() {
                // Find least recently used glyph
                if let Some((&lru_char, _)) = self.glyphs.iter()
                    .min_by_key(|(_, g)| g.last_access) {
                    if let Some(removed) = self.glyphs.remove(&lru_char) {
                        self.cache_size -= removed.data.len() + core::mem::size_of::<CachedGlyph>();
                    }
                }
            }

            // Use advance from image placement
            let advance = img.placement.width as i32 + 1; // Add 1px spacing

            self.access_counter = self.access_counter.wrapping_add(1);
            let cached = CachedGlyph {
                data: img.data.clone(),
                width: img.placement.width as usize,
                height: img.placement.height as usize,
                left: img.placement.left,
                top: img.placement.top,
                advance,
                last_access: self.access_counter,
            };

            self.cache_size += glyph_size;
            self.glyphs.insert(ch, cached);
            return self.glyphs.get(&ch);
        }

        None
    }

    /// Draw text at the specified position with the given color
    /// Returns (width, height) of the drawn text in pixels
    pub fn draw_text<T: RasterTarget>(
        &mut self,
        target: &mut T,
        text: &str,
        x: i32,
        y: i32,
        color: Color,
    ) -> (f32, f32) {
        let mut cursor_x = x;
        let start_x = x as f32;
        let mut max_height = 0.0f32;

        for ch in text.chars() {
            if let Some(glyph) = self.get_or_cache_glyph(ch) {
                // Track maximum height
                let glyph_height = (glyph.height as i32 + glyph.top) as f32;
                max_height = max_height.max(glyph_height);

                // Draw glyph with baseline alignment using RasterTarget
                for gy in 0..glyph.height {
                    let py = y - glyph.top + gy as i32;
                    if py < 0 || py >= target.height() as i32 {
                        continue;
                    }

                    let px_start = cursor_x + glyph.left;
                    if px_start >= target.width() as i32 {
                        continue;
                    }

                    // Extract coverage for this row
                    let row_start = gy * glyph.width;
                    let coverage = &glyph.data[row_start..row_start + glyph.width];

                    // Clip to visible range
                    let clip_start = if px_start < 0 { -px_start as usize } else { 0 };
                    let visible_width = (glyph.width - clip_start)
                        .min((target.width() as i32 - (px_start + clip_start as i32)) as usize);

                    if visible_width > 0 {
                        let x_coord = (px_start + clip_start as i32) as u16;
                        target.blend_solid_hspan(
                            py as u16,
                            x_coord,
                            color,
                            &coverage[clip_start..clip_start + visible_width],
                        );
                    }
                }
                cursor_x += glyph.advance;
            } else if ch == ' ' {
                cursor_x += (self.font_size * 0.3) as i32;
                max_height = max_height.max(self.font_size);
            }
        }
        
        (cursor_x as f32 - start_x, max_height)
    }

    /// Get the baseline offset for this font
    pub fn baseline(&self) -> i32 {
        self.baseline
    }

    /// Get the font size
    pub fn size(&self) -> f32 {
        self.font_size
    }

    /// Calculate the width of a text string in pixels
    pub fn text_width(&self, text: &str) -> f32 {
        self.text_dimensions(text).0
    }

    /// Calculate the dimensions (width, height) of a text string in pixels
    pub fn text_dimensions(&self, text: &str) -> (f32, f32) {
        let mut width = 0.0f32;
        let mut max_height = 0.0f32;
        
        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
                width += glyph.advance as f32;
                let glyph_height = (glyph.height as i32 + glyph.top) as f32;
                max_height = max_height.max(glyph_height);
            } else if ch == ' ' {
                width += self.font_size * 0.3;
                max_height = max_height.max(self.font_size);
            }
        }
        (width, max_height)
    }
}

/// Builder for creating fonts with LRU-cached glyphs
pub struct FontBuilder {
    font_data: Option<&'static [u8]>,
    font_size: f32,
    cache_chars: Option<&'static str>,
    hint: bool,
    max_cache_size: usize,
}

impl FontBuilder {
    fn new() -> Self {
        Self {
            font_data: None,
            font_size: 12.0,
            cache_chars: None,
            hint: true,
            max_cache_size: 4096, // 4KB default
        }
    }

    /// Set the font data (TrueType/OpenType bytes)
    pub fn data(mut self, data: &'static [u8]) -> Self {
        self.font_data = Some(data);
        self
    }

    /// Set the font size in pixels
    pub fn size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the characters to pre-cache
    pub fn cache(mut self, chars: &'static str) -> Self {
        self.cache_chars = Some(chars);
        self
    }

    /// Enable or disable hinting
    pub fn hint(mut self, enable: bool) -> Self {
        self.hint = enable;
        self
    }

    /// Set maximum cache size in bytes (default: 4096)
    pub fn max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    /// Build the font with the specified configuration
    pub fn build(self) -> Font {
        let font_data = self.font_data.expect("Font data is required");
        let font_ref = FontRef::from_index(font_data, 0).expect("Failed to load font");

        let mut scale_context = ScaleContext::new();
        let mut scaler = scale_context
            .builder(font_ref)
            .size(self.font_size)
            .hint(self.hint)
            .build();

        // Get font metrics to calculate baseline
        let metrics = font_ref.metrics(&[]).scale(self.font_size);
        let baseline = metrics.ascent as i32;

        let charmap = font_ref.charmap();
        let mut glyphs = BTreeMap::new();
        let mut cache_size = 0;

        // Pre-cache specified characters or use minimal default set
        let chars_to_cache = self.cache_chars.unwrap_or("0123456789");

        for ch in chars_to_cache.chars() {
            let glyph_id = charmap.map(ch);

            if let Some(img) = Render::new(&[Source::Outline]).render(&mut scaler, glyph_id) {
                if let Content::Mask = img.content {
                    let glyph_size = img.data.len() + core::mem::size_of::<CachedGlyph>();
                    
                    // Respect cache size limit
                    if cache_size + glyph_size > self.max_cache_size {
                        break;
                    }

                    // Use advance from image placement
                    let advance = img.placement.width as i32 + 1; // Add 1px spacing

                    let cached = CachedGlyph {
                        data: img.data,
                        width: img.placement.width as usize,
                        height: img.placement.height as usize,
                        left: img.placement.left,
                        top: img.placement.top,
                        advance,
                        last_access: 0,
                    };

                    cache_size += glyph_size;
                    glyphs.insert(ch, cached);
                }
            }
        }

        Font {
            glyphs,
            font_data,
            font_size: self.font_size,
            baseline,
            hint: self.hint,
            cache_size,
            max_cache_size: self.max_cache_size,
            access_counter: 0,
        }
    }
}
