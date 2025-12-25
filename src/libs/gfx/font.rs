use font8x8::legacy::BASIC_LEGACY;

/// Bitmap data for a single monospaced glyph.
pub struct GlyphBitmap<'a> {
    data: &'a [u8],
    height: u8,
    width: u8,
    bytes_per_row: u8,
}

impl<'a> GlyphBitmap<'a> {
    #[inline(always)]
    pub fn height(&self) -> u8 {
        self.height
    }

    #[inline(always)]
    pub fn width(&self) -> u8 {
        self.width
    }

    #[inline(always)]
    pub fn is_pixel_inked(&self, x: u8, y: u8) -> bool {
        let stride = self.bytes_per_row as usize;
        let offset = y as usize * stride + (x as usize / 8);
        let row = self.data[offset];
        let bit = 7 - (x & 7);
        (row >> bit) & 1 == 1
    }
}

/// Metadata and bitmap store for a monospaced font.
pub struct MonoFont {
    bitmap: &'static [u8],
    glyph_height: u8,
    glyph_width: u8,
    bytes_per_row: u8,
    first_codepoint: u32,
    glyph_count: u16,
    baseline: u8,
    advance: u8,
    tracking: i8,
}

impl MonoFont {
    pub const fn new(
        bitmap: &'static [u8],
        glyph_width: u8,
        glyph_height: u8,
        first_codepoint: u32,
        glyph_count: u16,
        baseline: u8,
        advance: u8,
        tracking: i8,
    ) -> Self {
        let bytes_per_row = ((glyph_width as usize + 7) / 8) as u8;
        Self {
            bitmap,
            glyph_height,
            glyph_width,
            bytes_per_row,
            first_codepoint,
            glyph_count,
            baseline,
            advance,
            tracking,
        }
    }

    #[inline(always)]
    pub fn glyph(&self, ch: char) -> Option<GlyphBitmap<'_>> {
        let code = ch as u32;
        if code < self.first_codepoint {
            return None;
        }

        let idx = code - self.first_codepoint;
        if idx >= self.glyph_count as u32 {
            return None;
        }

        let glyph_stride = self.bytes_per_row as usize * self.glyph_height as usize;
        let start = idx as usize * glyph_stride;
        let end = start + glyph_stride;

        Some(GlyphBitmap {
            data: &self.bitmap[start..end],
            height: self.glyph_height,
            width: self.glyph_width,
            bytes_per_row: self.bytes_per_row,
        })
    }

    #[inline(always)]
    pub fn height(&self) -> u8 {
        self.glyph_height
    }

    #[inline(always)]
    pub fn width(&self) -> u8 {
        self.glyph_width
    }

    #[inline(always)]
    pub fn baseline(&self) -> u8 {
        self.baseline
    }

    #[inline(always)]
    pub fn advance(&self) -> u8 {
        self.advance
    }

    #[inline(always)]
    pub fn tracking(&self) -> i8 {
        self.tracking
    }
}

const FIRST_PRINTABLE: u32 = 0x20;
const PRINTABLE_COUNT: usize = 95;
const GLYPH_HEIGHT: usize = 8;

const fn reverse_bits(mut byte: u8) -> u8 {
    let mut out = 0u8;
    let mut i = 0;
    while i < 8 {
        out = (out << 1) | (byte & 1);
        byte >>= 1;
        i += 1;
    }
    out
}

const fn flatten_basic_font() -> [u8; PRINTABLE_COUNT * GLYPH_HEIGHT] {
    let mut data = [0u8; PRINTABLE_COUNT * GLYPH_HEIGHT];
    let mut glyph = 0;

    while glyph < PRINTABLE_COUNT {
        let source = BASIC_LEGACY[(FIRST_PRINTABLE as usize) + glyph];
        let mut row = 0;
        while row < GLYPH_HEIGHT {
            data[glyph * GLYPH_HEIGHT + row] = reverse_bits(source[row]);
            row += 1;
        }
        glyph += 1;
    }

    data
}

const SYSTEM_FONT_BITMAP: [u8; PRINTABLE_COUNT * GLYPH_HEIGHT] = flatten_basic_font();

/// Default 8x8 ASCII monospaced font generated from the `font8x8` BASIC table.
pub const SYSTEM_MONO_FONT: MonoFont = MonoFont::new(
    &SYSTEM_FONT_BITMAP,
    8,
    8,
    FIRST_PRINTABLE,
    PRINTABLE_COUNT as u16,
    7,
    8,
    1,
);
