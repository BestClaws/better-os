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
}

/// Font renderer with pre-cached glyphs
pub struct Font {
    glyphs: BTreeMap<char, CachedGlyph>,
    font_size: f32,
    baseline: i32,
}

impl Font {
    /// Create a new font builder
    pub fn builder() -> FontBuilder {
        FontBuilder::new()
    }

    /// Draw text at the specified position with the given color
    pub fn draw_text<T: RasterTarget>(
        &self,
        target: &mut T,
        text: &str,
        x: i32,
        y: i32,
        color: Color,
    ) {
        let mut cursor_x = x;

        for ch in text.chars() {
            if let Some(glyph) = self.glyphs.get(&ch) {
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
            }
        }
    }

    /// Get the baseline offset for this font
    pub fn baseline(&self) -> i32 {
        self.baseline
    }

    /// Get the font size
    pub fn size(&self) -> f32 {
        self.font_size
    }
}

/// Builder for creating fonts with pre-cached glyphs
pub struct FontBuilder {
    font_data: Option<&'static [u8]>,
    font_size: f32,
    cache_chars: Option<&'static str>,
    hint: bool,
}

impl FontBuilder {
    fn new() -> Self {
        Self {
            font_data: None,
            font_size: 12.0,
            cache_chars: None,
            hint: true,
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

        // Cache specified characters or use default set
        let chars_to_cache = self
            .cache_chars
            .unwrap_or("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 !?.:,'-");

        for ch in chars_to_cache.chars() {
            let glyph_id = charmap.map(ch);

            if let Some(img) = Render::new(&[Source::Outline]).render(&mut scaler, glyph_id) {
                if let Content::Mask = img.content {
                    let cached = CachedGlyph {
                        data: img.data,
                        width: img.placement.width as usize,
                        height: img.placement.height as usize,
                        left: img.placement.left,
                        top: img.placement.top,
                        advance: img.placement.width as i32 + 2,
                    };

                    glyphs.insert(ch, cached);
                }
            }
        }

        Font {
            glyphs,
            font_size: self.font_size,
            baseline,
        }
    }
}
