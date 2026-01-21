//! Helpers for computing label glyph layout and metrics.

extern crate alloc;

use alloc::vec::Vec;
use core::iter::Peekable;
use core::str::Chars;

use crate::types::Point;

use super::font::{glyph_for_char, kerning, Font, Glyph};

#[derive(Clone, Debug)]
pub(crate) struct GlyphPlacement {
    pub glyph: &'static Glyph,
    pub glyph_id: u16,
    pub origin: Point,
}

#[derive(Clone, Debug)]
pub(crate) struct LineLayout {
    pub glyphs: Vec<GlyphPlacement>,
    pub baseline_y: i32,
    pub pen_start_x: i32,
    pub pen_end_x: i32,
}

pub(crate) fn layout_single_line(
    text: &str,
    letter_space: i32,
    font: &'static Font,
    pen_start_x: i32,
    baseline_y: i32,
) -> Option<LineLayout> {
    let mut glyphs = Vec::new();
    let mut pen_x = pen_start_x;
    let mut prev_glyph_id: Option<u16> = None;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        let Some(info) = glyph_for_char(font, ch) else {
            prev_glyph_id = None;
            continue;
        };

        if let Some(prev_id) = prev_glyph_id {
            pen_x += kerning(font, prev_id, info.glyph_id);
        }

        let glyph_x = pen_x + info.glyph.ofs_x as i32;
        let glyph_y = baseline_y - info.glyph.box_h as i32 - info.glyph.ofs_y as i32;

        glyphs.push(GlyphPlacement {
            glyph: info.glyph,
            glyph_id: info.glyph_id,
            origin: Point::new(glyph_x, glyph_y),
        });

        pen_x += advance_px(info.glyph);
        if chars.peek().is_some() {
            pen_x += letter_space;
        }
        prev_glyph_id = Some(info.glyph_id);
    }

    if glyphs.is_empty() {
        return None;
    }

    Some(LineLayout {
        glyphs,
        baseline_y,
        pen_start_x,
        pen_end_x: pen_x,
    })
}

pub(crate) fn measure_text_for_font(font: &'static Font, text: &str, letter_space: i32) -> i32 {
    let mut width = 0;
    let mut prev_glyph_id: Option<u16> = None;
    let mut chars: Peekable<Chars<'_>> = text.chars().peekable();
    let mut measured_any = false;

    while let Some(ch) = chars.next() {
        let Some(info) = glyph_for_char(font, ch) else {
            prev_glyph_id = None;
            continue;
        };

        measured_any = true;

        if let Some(prev_id) = prev_glyph_id {
            width += kerning(font, prev_id, info.glyph_id);
        }

        width += advance_px(info.glyph);

        if chars.peek().is_some() {
            width += letter_space;
        }

        prev_glyph_id = Some(info.glyph_id);
    }

    if measured_any {
        width
    } else {
        0
    }
}

#[inline]
pub(crate) fn advance_px(glyph: &Glyph) -> i32 {
    ((glyph.adv_w_raw as i32 + 8) >> 4)
}
