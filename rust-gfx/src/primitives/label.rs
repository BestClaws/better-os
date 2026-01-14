mod label_font;

use crate::color::Rgba8888;
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;
use crate::Rasterizer;

use label_font::{glyph_for_char, kerning, FontMetrics, METRICS};

/// Text decoration
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TextDecor {
    None,
    Underline,
    Strikethrough,
}

/// Label descriptor matching LVGL
#[derive(Clone, Debug)]
pub struct LabelDsc {
    pub text: String,
    pub color: Rgba8888,
    pub opa: Opa,
    pub decor: TextDecor,
    pub letter_space: i32,
}

impl LabelDsc {
    pub fn new(text: String) -> Self {
        Self {
            text,
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            decor: TextDecor::None,
            letter_space: 0,
        }
    }
}

pub fn draw_label<R: Rasterizer>(rast: &mut R, dsc: &LabelDsc, area: &Area) {
    if dsc.opa == 0 || dsc.text.is_empty() {
        return;
    }

    let chars: Vec<char> = dsc.text.chars().collect();
    if chars.is_empty() {
        return;
    }

    let FontMetrics {
        line_height,
        base_line,
        underline_position,
        underline_thickness,
    } = METRICS;

    let mut cursor_x = area.x1;
    let line_start_x = cursor_x;
    let baseline_y = area.y1 + line_height - base_line;
    let mut line_end_x = cursor_x;

    for (idx, ch) in chars.iter().enumerate() {
        let glyph = match glyph_for_char(*ch) {
            Some(g) => g,
            None => continue,
        };

        let glyph_x = cursor_x + glyph.ofs_x as i32;
        let glyph_y = baseline_y - glyph.box_h as i32 - glyph.ofs_y as i32;

        let bitmap_width = glyph.box_w as usize;
        let bitmap_height = glyph.box_h as usize;
        let bitmap = glyph.bitmap;

        for row in 0..bitmap_height {
            for col in 0..bitmap_width {
                let coverage = bitmap[row * bitmap_width + col];
                if coverage == 0 {
                    continue;
                }
                let blended = mul_opa(coverage, dsc.opa);
                if blended == 0 {
                    continue;
                }
                let px = glyph_x + col as i32;
                let py = glyph_y + row as i32;
                rast.blend_pixel(px, py, dsc.color, blended);
            }
        }

        let kern_raw = chars
            .get(idx + 1)
            .map(|next| kerning(*ch, *next) as i32)
            .unwrap_or(0);
        let advance_raw = glyph.adv_w_raw as i32 + kern_raw;
        let advance_px = (advance_raw + 8) >> 4;
        cursor_x += advance_px;
        if idx + 1 < chars.len() {
            cursor_x += dsc.letter_space;
        }
        let glyph_end_x = glyph_x + glyph.box_w as i32;
        line_end_x = line_end_x.max(glyph_end_x).max(cursor_x);
    }

    if matches!(dsc.decor, TextDecor::None) || line_end_x <= line_start_x {
        return;
    }

    let thickness = underline_thickness.max(1);
    match dsc.decor {
        TextDecor::Underline => {
            let y = area.y1 + line_height - base_line - underline_position;
            draw_decoration_line(rast, line_start_x, line_end_x - 1, y, thickness, dsc);
        }
        TextDecor::Strikethrough => {
            let y = area.y1 + (line_height - base_line) * 2 / 3 + underline_thickness / 2;
            draw_decoration_line(rast, line_start_x, line_end_x - 1, y, thickness, dsc);
        }
        TextDecor::None => {}
    }
}

fn draw_decoration_line<R: Rasterizer>(
    rast: &mut R,
    x1: i32,
    x2: i32,
    y: i32,
    thickness: i32,
    dsc: &LabelDsc,
) {
    if dsc.opa == 0 || x2 < x1 {
        return;
    }
    for dy in 0..thickness {
        let py = y + dy;
        for px in x1..=x2 {
            rast.blend_pixel(px, py, dsc.color, dsc.opa);
        }
    }
}

#[inline]
fn mul_opa(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let prod = (a as u32) * (b as u32);
    ((prod + 127) / 255) as u8
}

/// Compute the pixel width of the provided text using the embedded font.
pub fn measure_text(text: &str, letter_space: i32) -> i32 {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut width = 0;
    for (idx, ch) in chars.iter().enumerate() {
        let glyph = match glyph_for_char(*ch) {
            Some(g) => g,
            None => continue,
        };

        let kern_raw = chars
            .get(idx + 1)
            .map(|next| kerning(*ch, *next) as i32)
            .unwrap_or(0);
        let advance_raw = glyph.adv_w_raw as i32 + kern_raw;
        let advance_px = (advance_raw + 8) >> 4;
        width += advance_px;
        if idx + 1 < chars.len() {
            width += letter_space;
        }
    }

    width
}

/// Returns the baseline-to-baseline height of the embedded font.
pub fn line_height() -> i32 {
    METRICS.line_height
}
