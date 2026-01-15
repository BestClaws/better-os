use crate::system::ui::drawing_surface::DrawingSurface;
use font8x8::{UnicodeFonts, BASIC_FONTS};
use rust_gfx::color::Rgba8888;

pub const FONT_HEIGHT: i32 = 8;
pub const CHAR_WIDTH: i32 = 8;
pub const SPACE_WIDTH: i32 = 4;

pub fn ascii_text_width(text: &str) -> i32 {
    text.chars()
        .map(|ch| if ch == ' ' { SPACE_WIDTH } else { CHAR_WIDTH })
        .sum()
}

pub fn draw_ascii_text(surface: &mut DrawingSurface, text: &str, x: i32, y: i32, color: Rgba8888) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += SPACE_WIDTH;
            continue;
        }
        let glyph = match BASIC_FONTS.get(ch) {
            Some(g) => g,
            None => {
                cursor_x += CHAR_WIDTH;
                continue;
            }
        };
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 == 1 {
                    surface.set_pixel_internal(cursor_x + col as i32, y + row as i32, color);
                }
            }
        }
        cursor_x += CHAR_WIDTH;
    }
}
