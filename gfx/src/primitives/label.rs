mod label_font;

use crate::colors::Color;
extern crate alloc;
use alloc::vec;
use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;
use crate::RasterTarget;

use label_font::{default_font, font, glyph_for_char, kerning, Font};

pub use label_font::FontId;

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
    pub color: Color,
    pub opa: Opacity,
    pub decor: TextDecor,
    pub letter_space: i32,
    pub font: FontId,
}

impl LabelDsc {
    pub fn new(text: String) -> Self {
        Self {
            text,
            color: Color::WHITE,
            opa: OPA100,
            decor: TextDecor::None,
            letter_space: 0,
            font: FontId::Montserrat14,
        }
    }
}

pub fn draw_label<R: RasterTarget>(rast: &mut R, dsc: &LabelDsc, area: &Area) {
    if dsc.opa == 0 || dsc.text.is_empty() {
        return;
    }

    let chars: Vec<char> = dsc.text.chars().collect();
    if chars.is_empty() {
        return;
    }

    let font = font(dsc.font);
    let line_height = font.line_height;
    let base_line = font.base_line;
    let underline_position = font.underline_position;
    let underline_thickness = font.underline_thickness;

    let mut cursor_x = area.x1;
    let line_start_x = cursor_x;
    let baseline_y = area.y1 + line_height - base_line;
    let mut line_end_x = cursor_x;
    let mut prev_glyph_id: Option<u16> = None;

    for (idx, ch) in chars.iter().enumerate() {
        let glyph_info = match glyph_for_char(font, *ch) {
            Some(g) => g,
            None => {
                prev_glyph_id = None;
                continue;
            }
        };

        if let Some(prev_id) = prev_glyph_id {
            let kern_raw = kerning(font, prev_id, glyph_info.glyph_id) as i32;
            cursor_x += ((kern_raw + 8) >> 4);
        }

        let glyph = glyph_info.glyph;

        let glyph_x = cursor_x + glyph.ofs_x as i32;
        let glyph_y = baseline_y - glyph.box_h as i32 - glyph.ofs_y as i32;

        let bitmap_width = glyph.box_w as usize;
        let bitmap_height = glyph.box_h as usize;
        let start = glyph.bitmap_offset as usize;
        let end = start + glyph.bitmap_len as usize;
        let bitmap = &font.bitmap[start..end];

        if bitmap.len() != bitmap_width * bitmap_height {
            prev_glyph_id = Some(glyph_info.glyph_id);
            continue;
        }

        if bitmap_width > 0 && bitmap_height > 0 {
            let mut coverage_row = vec![0u8; bitmap_width];

            for row in 0..bitmap_height {
                coverage_row.fill(0);
                let mut any = false;
                let row_slice = &bitmap[row * bitmap_width..(row + 1) * bitmap_width];

                for (col, &coverage) in row_slice.iter().enumerate() {
                    if coverage == 0 {
                        continue;
                    }
                    let blended = mul_opa(coverage, dsc.opa);
                    if blended == 0 {
                        continue;
                    }
                    coverage_row[col] = blended;
                    any = true;
                }

                if any {
                    let py = glyph_y + row as i32;
                    rast.blend_solid_hspan(glyph_x, py, dsc.color, &coverage_row);
                }
            }
        }

        let advance_px = (glyph.adv_w_raw as i32 + 8) >> 4;
        cursor_x += advance_px;
        if idx + 1 < chars.len() {
            cursor_x += dsc.letter_space;
        }
        let glyph_end_x = glyph_x + glyph.box_w as i32;
        line_end_x = line_end_x.max(glyph_end_x).max(cursor_x);
        prev_glyph_id = Some(glyph_info.glyph_id);
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

fn draw_decoration_line<R: RasterTarget>(
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
    let span_len = x2 - x1 + 1;
    for dy in 0..thickness {
        let py = y + dy;
        if dsc.opa == OPA100 {
            rast.fill_rect(x1, py, span_len, 1, dsc.color);
        } else {
            rast.blend_hspan_with(x1, py, span_len, |_| (dsc.color, dsc.opa));
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
    measure_text_with_font(text, letter_space, FontId::Montserrat14)
}

pub fn measure_text_with_font(text: &str, letter_space: i32, font_id: FontId) -> i32 {
    let font = font(font_id);
    measure_text_for_font(font, text, letter_space)
}

/// Return the baseline-to-baseline height for the given font.
pub fn line_height_for_font(font_id: FontId) -> i32 {
    font(font_id).line_height
}

/// Return the default font identifier used by labels.
pub fn default_font_id() -> FontId {
    FontId::Montserrat14
}

/// Returns the baseline-to-baseline height of the embedded font.
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
        let glyph_info = match glyph_for_char(font, *ch) {
            Some(g) => g,
            None => {
                prev_glyph_id = None;
                continue;
            }
        };

        if let Some(prev_id) = prev_glyph_id {
            let kern_raw = kerning(font, prev_id, glyph_info.glyph_id) as i32;
            width += (kern_raw + 8) >> 4;
        }

        width += (glyph_info.glyph.adv_w_raw as i32 + 8) >> 4;
        if idx + 1 < chars.len() {
            width += letter_space;
        }

        prev_glyph_id = Some(glyph_info.glyph_id);
    }

    width
}
