//! Embedded font definitions used by the label primitive.

mod data;

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

#[derive(Debug)]
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
    use data::{
        montserrat_10, montserrat_12, montserrat_14, montserrat_16, montserrat_18, montserrat_20,
        montserrat_22, montserrat_24, montserrat_26, montserrat_28, montserrat_30, montserrat_32,
        montserrat_34, montserrat_36, montserrat_38, montserrat_40, montserrat_42, montserrat_44,
        montserrat_46, montserrat_48, montserrat_8,
    };

    match id {
        FontId::Montserrat8 => &montserrat_8::FONT,
        FontId::Montserrat10 => &montserrat_10::FONT,
        FontId::Montserrat12 => &montserrat_12::FONT,
        FontId::Montserrat14 => &montserrat_14::FONT,
        FontId::Montserrat16 => &montserrat_16::FONT,
        FontId::Montserrat18 => &montserrat_18::FONT,
        FontId::Montserrat20 => &montserrat_20::FONT,
        FontId::Montserrat22 => &montserrat_22::FONT,
        FontId::Montserrat24 => &montserrat_24::FONT,
        FontId::Montserrat26 => &montserrat_26::FONT,
        FontId::Montserrat28 => &montserrat_28::FONT,
        FontId::Montserrat30 => &montserrat_30::FONT,
        FontId::Montserrat32 => &montserrat_32::FONT,
        FontId::Montserrat34 => &montserrat_34::FONT,
        FontId::Montserrat36 => &montserrat_36::FONT,
        FontId::Montserrat38 => &montserrat_38::FONT,
        FontId::Montserrat40 => &montserrat_40::FONT,
        FontId::Montserrat42 => &montserrat_42::FONT,
        FontId::Montserrat44 => &montserrat_44::FONT,
        FontId::Montserrat46 => &montserrat_46::FONT,
        FontId::Montserrat48 => &montserrat_48::FONT,
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

pub fn kerning(font: &'static Font, left_id: u16, right_id: u16) -> i32 {
    let Some(k) = font.kerning else {
        return 0;
    };

    let left_idx = left_id as usize;
    let right_idx = right_id as usize;

    if left_idx == 0
        || right_idx == 0
        || left_idx >= k.left_class_mapping.len()
        || right_idx >= k.right_class_mapping.len()
    {
        return 0;
    }

    let left_class = k.left_class_mapping[left_idx];
    let right_class = k.right_class_mapping[right_idx];

    if left_class == 0 || right_class == 0 {
        return 0;
    }

    let pair_index =
        (left_class as usize - 1) * k.right_class_cnt as usize + (right_class as usize - 1);
    let raw = k.class_pair_values.get(pair_index).copied().unwrap_or(0) as i32;

    if raw == 0 {
        return 0;
    }

    // LVGL stores kerning in 12.4 fixed-point; convert to the Q4 delta.
    let kv_q4 = (raw * k.scale as i32) >> 4;
    if kv_q4 == 0 {
        return 0;
    }

    let Some(left_glyph) = font.glyphs.get(left_idx) else {
        return 0;
    };

    let adv_raw = left_glyph.adv_w_raw as i32;
    // Mirror LVGL's rounding when folding kerning into the advance width.
    let adv_px_with = (adv_raw + kv_q4 + 8) >> 4;
    let adv_px_without = (adv_raw + 8) >> 4;

    adv_px_with - adv_px_without
}

pub fn glyph_bitmap<'font>(font: &'font Font, glyph: &'font Glyph) -> &'font [u8] {
    let start = glyph.bitmap_offset as usize;
    let end = start + glyph.bitmap_len as usize;
    &font.bitmap[start..end]
}
