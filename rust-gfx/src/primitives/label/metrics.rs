//! Public helpers for working with label text metrics.

use super::font::{default_font, default_font_id, font, FontId};
use super::layout;

/// Compute the pixel width of the provided text using the default font.
pub fn measure_text(text: &str, letter_space: i32) -> i32 {
    measure_text_with_font(text, letter_space, default_font_id())
}

/// Compute the pixel width of the provided text using a specific font.
pub fn measure_text_with_font(text: &str, letter_space: i32, font_id: FontId) -> i32 {
    let font = font(font_id);
    layout::measure_text_for_font(font, text, letter_space)
}

/// Return the baseline-to-baseline height for the given font.
pub fn line_height_for_font(font_id: FontId) -> i32 {
    font(font_id).line_height
}

/// Return the baseline-to-baseline height of the default font.
pub fn line_height() -> i32 {
    default_font().line_height
}
