// Auto-generated LVGL font data loader. Do not edit by hand.

extern crate alloc;

use alloc::vec::Vec;

mod fonts;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum FontId {
    Montserrat8,
    Montserrat10,
    Montserrat12,
    Montserrat14,
    Montserrat16,
    Montserrat18,
    Montserrat20,
    Montserrat22,
    Montserrat24,
    Montserrat26,
    Montserrat28,
    Montserrat30,
    Montserrat32,
    Montserrat34,
    Montserrat36,
    Montserrat38,
    Montserrat40,
    Montserrat42,
    Montserrat44,
    Montserrat46,
    Montserrat48,
}

pub struct Glyph {
    pub adv_w_raw: u16,
    pub box_w: u8,
    pub box_h: u8,
    pub ofs_x: i8,
    pub ofs_y: i8,
    pub bitmap_offset: u32,
    pub bitmap_len: u16,
}

pub struct GlyphMap {
    pub ch: char,
    pub glyph_index: u16,
}

pub struct FontKerning {
    pub left_class_mapping: &'static [u8],
    pub right_class_mapping: &'static [u8],
    pub class_pair_values: &'static [i8],
    pub left_class_cnt: u8,
    pub right_class_cnt: u8,
    pub scale: u8,
}

pub struct Font {
    pub id: FontId,
    pub name: &'static str,
    pub line_height: i32,
    pub base_line: i32,
    pub underline_position: i32,
    pub underline_thickness: i32,
    pub glyphs: &'static [Glyph],
    pub glyph_map: &'static [GlyphMap],
    pub bitmap: &'static [u8],
    pub kerning: Option<&'static FontKerning>,
}

pub struct GlyphInfo {
    pub glyph: &'static Glyph,
    pub glyph_id: u16,
}

pub fn font(id: FontId) -> &'static Font {
    match id {
        FontId::Montserrat8 => &fonts::montserrat_8::FONT,
        FontId::Montserrat10 => &fonts::montserrat_10::FONT,
        FontId::Montserrat12 => &fonts::montserrat_12::FONT,
        FontId::Montserrat14 => &fonts::montserrat_14::FONT,
        FontId::Montserrat16 => &fonts::montserrat_16::FONT,
        FontId::Montserrat18 => &fonts::montserrat_18::FONT,
        FontId::Montserrat20 => &fonts::montserrat_20::FONT,
        FontId::Montserrat22 => &fonts::montserrat_22::FONT,
        FontId::Montserrat24 => &fonts::montserrat_24::FONT,
        FontId::Montserrat26 => &fonts::montserrat_26::FONT,
        FontId::Montserrat28 => &fonts::montserrat_28::FONT,
        FontId::Montserrat30 => &fonts::montserrat_30::FONT,
        FontId::Montserrat32 => &fonts::montserrat_32::FONT,
        FontId::Montserrat34 => &fonts::montserrat_34::FONT,
        FontId::Montserrat36 => &fonts::montserrat_36::FONT,
        FontId::Montserrat38 => &fonts::montserrat_38::FONT,
        FontId::Montserrat40 => &fonts::montserrat_40::FONT,
        FontId::Montserrat42 => &fonts::montserrat_42::FONT,
        FontId::Montserrat44 => &fonts::montserrat_44::FONT,
        FontId::Montserrat46 => &fonts::montserrat_46::FONT,
        FontId::Montserrat48 => &fonts::montserrat_48::FONT,
    }
}

pub fn default_font_id() -> FontId {
    FontId::Montserrat14
}

pub fn default_font() -> &'static Font {
    font(default_font_id())
}

pub fn glyph_for_char(font: &'static Font, ch: char) -> Option<GlyphInfo> {
    match font.glyph_map.binary_search_by(|entry| entry.ch.cmp(&ch)) {
        Ok(idx) => {
            let entry = &font.glyph_map[idx];
            font.glyphs
                .get(entry.glyph_index as usize)
                .map(|glyph| GlyphInfo {
                    glyph,
                    glyph_id: entry.glyph_index,
                })
        }
        Err(_) => None,
    }
}

pub fn kerning(font: &'static Font, left_id: u16, right_id: u16) -> i16 {
    let Some(k) = font.kerning else {
        return 0;
    };
    let left = k
        .left_class_mapping
        .get(left_id as usize)
        .copied()
        .unwrap_or(0);
    let right = k
        .right_class_mapping
        .get(right_id as usize)
        .copied()
        .unwrap_or(0);
    if left == 0 || right == 0 {
        return 0;
    }
    let idx = (left as usize - 1) * k.right_class_cnt as usize + (right as usize - 1);
    let raw = k.class_pair_values.get(idx).copied().unwrap_or(0) as i16;
    ((raw as i32 * k.scale as i32) >> 4) as i16
}

pub fn measure_text(text: &str, letter_space: i32) -> i32 {
    measure_text_with_font(text, letter_space, default_font_id())
}

pub fn measure_text_with_font(text: &str, letter_space: i32, font_id: FontId) -> i32 {
    let font = font(font_id);
    measure_text_for_font(font, text, letter_space)
}

pub fn line_height_for_font(font_id: FontId) -> i32 {
    font(font_id).line_height
}

pub fn line_height() -> i32 {
    default_font().line_height
}

fn measure_text_for_font(font: &'static Font, text: &str, letter_space: i32) -> i32 {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut width = 0;
    let mut prev_glyph_id: Option<u16> = None;

    for (idx, ch) in chars.iter().enumerate() {
        let Some(info) = glyph_for_char(font, *ch) else {
            prev_glyph_id = None;
            continue;
        };

        if let Some(prev) = prev_glyph_id {
            width += kerning(font, prev, info.glyph_id) as i32;
        }

        width += ((info.glyph.adv_w_raw as i32 + 8) >> 4);
        if idx + 1 < chars.len() {
            width += letter_space;
        }

        prev_glyph_id = Some(info.glyph_id);
    }

    width
}
