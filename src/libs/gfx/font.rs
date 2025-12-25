use core::cmp;

use font8x8::{
    legacy::NOTHING_TO_DISPLAY,
    unicode::{
        FontUnicode, BASIC_UNICODE, BLOCK_UNICODE, BOX_UNICODE, GREEK_UNICODE, HIRAGANA_UNICODE,
        LATIN_UNICODE, MISC_UNICODE, SGA_UNICODE,
    },
};

const BASE_DIMENSION: u8 = 8;
const BASE_BASELINE: u8 = 7;

/// Discrete font sizes that the UI can request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

/// Glyph tables that can be consulted for Unicode coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charset {
    Basic,
    Latin,
    Greek,
    Block,
    BoxDrawing,
    Misc,
    Hiragana,
    Sga,
}

impl Charset {
    #[inline(always)]
    fn lookup(self, ch: char) -> Option<[u8; 8]> {
        match self {
            Charset::Basic => lookup_in(&BASIC_UNICODE, ch),
            Charset::Latin => lookup_in(&LATIN_UNICODE, ch),
            Charset::Greek => lookup_in(&GREEK_UNICODE, ch),
            Charset::Block => lookup_in(&BLOCK_UNICODE, ch),
            Charset::BoxDrawing => lookup_in(&BOX_UNICODE, ch),
            Charset::Misc => lookup_in(&MISC_UNICODE, ch),
            Charset::Hiragana => lookup_in(&HIRAGANA_UNICODE, ch),
            Charset::Sga => lookup_in(&SGA_UNICODE, ch),
        }
    }
}

const DEFAULT_CHARSETS_ARRAY: [Charset; 8] = [
    Charset::Basic,
    Charset::Latin,
    Charset::Greek,
    Charset::Block,
    Charset::BoxDrawing,
    Charset::Misc,
    Charset::Hiragana,
    Charset::Sga,
];

/// Default lookup order for glyph tables.
pub const DEFAULT_CHARSETS: &'static [Charset] = &DEFAULT_CHARSETS_ARRAY;

/// Runtime glyph bitmap with an integer scale applied to the underlying 8x8 mask.
#[derive(Clone, Copy)]
pub struct GlyphBitmap {
    bitmap: [u8; 8],
    scale: u8,
}

impl GlyphBitmap {
    #[inline(always)]
    pub fn width(&self) -> u8 {
        BASE_DIMENSION * self.scale.max(1)
    }

    #[inline(always)]
    pub fn height(&self) -> u8 {
        BASE_DIMENSION * self.scale.max(1)
    }

    #[inline(always)]
    pub fn scale(&self) -> u8 {
        self.scale.max(1)
    }

    #[inline(always)]
    pub fn is_pixel_inked(&self, x: u8, y: u8) -> bool {
        let scale = self.scale().max(1) as usize;
        let base_x = (x as usize) / scale;
        let base_y = (y as usize) / scale;
        let row = self.bitmap[base_y];
        let shift = base_x as u8;
        ((row >> shift) & 1) == 1
    }
}

/// Monospaced bitmap font description with configurable unicode coverage and scale.
#[derive(Clone, Copy)]
pub struct MonoFont {
    scale: u8,
    letter_spacing: i8,
    baseline: u8,
    charsets: &'static [Charset],
}

impl MonoFont {
    pub const fn new(scale: u8, letter_spacing: i8, charsets: &'static [Charset]) -> Self {
        let clamped_scale = if scale == 0 { 1 } else { scale };
        Self {
            scale: clamped_scale,
            letter_spacing,
            baseline: BASE_BASELINE * clamped_scale,
            charsets,
        }
    }

    #[inline(always)]
    pub fn glyph(&self, ch: char) -> GlyphBitmap {
        GlyphBitmap {
            bitmap: fetch_raw_glyph(ch, self.charsets),
            scale: self.scale,
        }
    }

    #[inline(always)]
    pub fn height(&self) -> u8 {
        BASE_DIMENSION * self.scale
    }

    #[inline(always)]
    pub fn width(&self) -> u8 {
        BASE_DIMENSION * self.scale
    }

    #[inline(always)]
    pub fn baseline(&self) -> u8 {
        self.baseline
    }

    #[inline(always)]
    pub fn advance(&self) -> u8 {
        self.width()
    }

    #[inline(always)]
    pub fn letter_spacing(&self) -> i8 {
        self.letter_spacing
    }

    #[inline(always)]
    pub fn line_advance(&self) -> i32 {
        i32::from(self.height()) + i32::from(cmp::max(self.letter_spacing, 0))
    }

    #[inline(always)]
    pub fn charsets(&self) -> &'static [Charset] {
        self.charsets
    }

    pub fn with_charsets(mut self, charsets: &'static [Charset]) -> Self {
        self.charsets = charsets;
        self
    }

    pub fn with_letter_spacing(mut self, letter_spacing: i8) -> Self {
        self.letter_spacing = letter_spacing;
        self
    }

    pub const fn from_size(size: FontSize) -> MonoFont {
        match size {
            FontSize::Small => MONO_FONT_SMALL,
            FontSize::Medium => MONO_FONT_MEDIUM,
            FontSize::Large => MONO_FONT_LARGE,
        }
    }

    pub fn for_size_with_charsets(size: FontSize, charsets: &'static [Charset]) -> MonoFont {
        MonoFont::from_size(size).with_charsets(charsets)
    }
}

pub const MONO_FONT_SMALL: MonoFont = MonoFont::new(1, -1, DEFAULT_CHARSETS);
pub const MONO_FONT_MEDIUM: MonoFont = MonoFont::new(2, 0, DEFAULT_CHARSETS);
pub const MONO_FONT_LARGE: MonoFont = MonoFont::new(3, 0, DEFAULT_CHARSETS);

pub const fn font_for_size(size: FontSize) -> MonoFont {
    MonoFont::from_size(size)
}

fn lookup_in(table: &[FontUnicode], ch: char) -> Option<[u8; 8]> {
    table
        .binary_search_by_key(&ch, |glyph| glyph.char())
        .ok()
        .map(|idx| table[idx].byte_array())
}

fn fallback_glyph() -> [u8; 8] {
    lookup_in(&BASIC_UNICODE, '?').unwrap_or(NOTHING_TO_DISPLAY)
}

fn fetch_raw_glyph(ch: char, charsets: &[Charset]) -> [u8; 8] {
    for charset in charsets {
        if let Some(bits) = charset.lookup(ch) {
            return bits;
        }
    }

    if ch == '\n' || ch == '\r' {
        return NOTHING_TO_DISPLAY;
    }

    fallback_glyph()
}
