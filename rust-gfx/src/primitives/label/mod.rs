//! Text label primitive matching LVGL's rendering behaviour.

extern crate alloc;

use alloc::string::String;

use crate::color::Rgba8888;
use crate::types::{Area, Opa, OPA_COVER};
use crate::Rasterizer;

use self::font::{default_font_id, font, glyph_bitmap, Font};
use self::layout::{layout_single_line, GlyphPlacement, LineLayout};

pub mod font;
mod layout;
mod metrics;

pub use crate::types::TextDecor;
pub use font::FontId;
pub use metrics::{line_height, line_height_for_font, measure_text, measure_text_with_font};

/// Descriptor for drawing a label.
#[derive(Clone, Debug)]
pub struct LabelDsc {
    pub text: String,
    pub color: Rgba8888,
    pub opa: Opa,
    pub decor: TextDecor,
    pub letter_space: i32,
    pub font: FontId,
}

impl LabelDsc {
    pub fn new(text: String) -> Self {
        Self {
            text,
            color: Rgba8888::WHITE,
            opa: OPA_COVER,
            decor: TextDecor::None,
            letter_space: 0,
            font: default_font_id(),
        }
    }
}

/// Render a label inside the provided area.
pub fn draw_label<R: Rasterizer>(rast: &mut R, dsc: &LabelDsc, area: &Area) {
    if dsc.opa == 0 || dsc.text.is_empty() {
        return;
    }

    let font = font(dsc.font);
    let baseline_y = area.y1 + font.line_height - font.base_line;

    let Some(line) = layout_single_line(&dsc.text, dsc.letter_space, font, area.x1, baseline_y)
    else {
        return;
    };

    for glyph in &line.glyphs {
        render_glyph(rast, dsc, glyph, font);
    }

    draw_decoration(rast, dsc, font, &line);
}

fn render_glyph<R: Rasterizer>(
    rast: &mut R,
    dsc: &LabelDsc,
    placement: &GlyphPlacement,
    font: &Font,
) {
    let bitmap = glyph_bitmap(font, placement.glyph);
    let width = placement.glyph.box_w as usize;
    let height = placement.glyph.box_h as usize;

    if bitmap.len() != width * height {
        return;
    }

    for row in 0..height {
        let py = placement.origin.y + row as i32;
        for col in 0..width {
            let coverage = bitmap[row * width + col];
            if coverage == 0 {
                continue;
            }

            let opa = blend_opa(coverage, dsc.opa);
            if opa == 0 {
                continue;
            }

            let px = placement.origin.x + col as i32;
            rast.blend_pixel(px, py, dsc.color, opa);
        }
    }
}

fn draw_decoration<R: Rasterizer>(rast: &mut R, dsc: &LabelDsc, font: &Font, line: &LineLayout) {
    if matches!(dsc.decor, TextDecor::None) {
        return;
    }

    let thickness = font.underline_thickness.max(1);
    let baseline_offset = font.line_height - font.base_line;
    let line_top = line.baseline_y - baseline_offset;
    let span_end = line.pen_end_x.saturating_sub(1);

    match dsc.decor {
        TextDecor::Underline => {
            let y = line.baseline_y - font.underline_position;
            draw_decoration_span(rast, line.pen_start_x, span_end, y, thickness, dsc);
        }
        TextDecor::Strikethrough => {
            let y = line_top + baseline_offset * 2 / 3 + thickness / 2;
            draw_decoration_span(rast, line.pen_start_x, span_end, y, thickness, dsc);
        }
        TextDecor::None => {}
    }
}

fn draw_decoration_span<R: Rasterizer>(
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
fn blend_opa(coverage: u8, opa: u8) -> u8 {
    if coverage == 0 || opa == 0 {
        return 0;
    }

    let prod = coverage as u32 * opa as u32;
    ((prod + 127) / 255) as u8
}
