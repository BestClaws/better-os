use core::cmp;

use embedded_graphics::mono_font::{
    ascii::{FONT_6X9, FONT_8X13, FONT_9X18},
    MonoFont as EgMonoFont,
};

/// Discrete font sizes that the UI can request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSize {
    Small,
    Medium,
    Large,
}

/// Legacy charset enum kept for API compatibility.
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

/// Charset list retained for callers but unused by embedded-graphics fonts.
pub const DEFAULT_CHARSETS: &[Charset] = &[];

/// Monospaced font descriptor wrapping an embedded-graphics font.
#[derive(Clone, Copy)]
pub struct MonoFont {
    font: EgMonoFont<'static>,
    letter_spacing: i8,
}

impl MonoFont {
    pub const fn new(font: EgMonoFont<'static>, letter_spacing: i8) -> Self {
        let spacing = if letter_spacing < 0 {
            0
        } else {
            letter_spacing as u32
        };
        Self {
            font: EgMonoFont {
                character_spacing: spacing,
                ..font
            },
            letter_spacing,
        }
    }

    pub const fn embedded(&self) -> &EgMonoFont<'static> {
        &self.font
    }

    pub const fn with_charsets(self, _charsets: &'static [Charset]) -> Self {
        // Charset override is no longer relevant; method kept for API compatibility.
        self
    }

    pub const fn with_letter_spacing(self, letter_spacing: i8) -> Self {
        let spacing = if letter_spacing < 0 {
            0
        } else {
            letter_spacing as u32
        };
        Self {
            font: EgMonoFont {
                character_spacing: spacing,
                ..self.font
            },
            letter_spacing,
        }
    }

    pub const fn height(&self) -> u32 {
        self.font.character_size.height
    }

    pub const fn width(&self) -> u32 {
        self.font.character_size.width
    }

    pub const fn baseline(&self) -> u32 {
        self.font.baseline
    }

    pub const fn advance(&self) -> u32 {
        self.width()
    }

    pub const fn letter_spacing(&self) -> i8 {
        self.letter_spacing
    }

    pub fn line_advance(&self) -> i32 {
        self.height() as i32 + cmp::max(self.letter_spacing as i32, 0)
    }

    pub const fn from_size(size: FontSize) -> MonoFont {
        match size {
            FontSize::Small => MONO_FONT_SMALL,
            FontSize::Medium => MONO_FONT_MEDIUM,
            FontSize::Large => MONO_FONT_LARGE,
        }
    }

    pub const fn for_size_with_charsets(size: FontSize, charsets: &'static [Charset]) -> MonoFont {
        let base = MonoFont::from_size(size);
        base.with_charsets(charsets)
    }
}

pub const MONO_FONT_SMALL: MonoFont = MonoFont::new(FONT_6X9, -1);
pub const MONO_FONT_MEDIUM: MonoFont = MonoFont::new(FONT_8X13, 0);
pub const MONO_FONT_LARGE: MonoFont = MonoFont::new(FONT_9X18, 0);

pub const fn font_for_size(size: FontSize) -> MonoFont {
    MonoFont::from_size(size)
}

pub const fn font_for_size_with_charsets(size: FontSize, charsets: &'static [Charset]) -> MonoFont {
    MonoFont::for_size_with_charsets(size, charsets)
}
