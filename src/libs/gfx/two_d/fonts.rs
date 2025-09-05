#![no_std]

use crate::libs::gfx::two_d::text::{BitmapFont, CharMetrics, FontInfo};

/// Embedded bitmap font data for space-grade text rendering.
/// 
/// This module contains pre-generated bitmap font data optimized for
/// embedded systems with minimal memory usage and maximum performance.

/// 8x8 pixel bitmap font for basic text rendering.
/// 
/// This font provides a clean, readable 8x8 pixel font suitable for
/// embedded applications where space is at a premium.
pub static FONT_8X8: BitmapFont = BitmapFont {
    info: FontInfo {
        name: "8x8",
        size: 8,
        line_height: 10,
        baseline: 7,
        first_char: 32, // Space
        last_char: 126, // Tilde
    },
    metrics: &FONT_8X8_METRICS,
    bitmap_data: &FONT_8X8_DATA,
    atlas_width: 760, // 95 characters * 8 pixels
    atlas_height: 8,
};

/// Character metrics for 8x8 font.
static FONT_8X8_METRICS: [CharMetrics; 95] = [
    // Space (32)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // ! (33)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // " (34)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // # (35)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // $ (36)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // % (37)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // & (38)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // ' (39)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 3 },
    // ( (40)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 5 },
    // ) (41)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 5 },
    // * (42)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // + (43)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // , (44)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // - (45)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // . (46)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // / (47)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 0 (48)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 1 (49)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // 2 (50)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 3 (51)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 4 (52)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 5 (53)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 6 (54)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 7 (55)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 8 (56)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // 9 (57)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // : (58)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // ; (59)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // < (60)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // = (61)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // > (62)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // ? (63)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // @ (64)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // A (65)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // B (66)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // C (67)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // D (68)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // E (69)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // F (70)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // G (71)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // H (72)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // I (73)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // J (74)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // K (75)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // L (76)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // M (77)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // N (78)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // O (79)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // P (80)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // Q (81)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // R (82)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // S (83)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // T (84)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // U (85)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // V (86)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // W (87)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // X (88)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // Y (89)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // Z (90)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // [ (91)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 5 },
    // \ (92)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // ] (93)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 5 },
    // ^ (94)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // _ (95)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // ` (96)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // a (97)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // b (98)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // c (99)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // d (100)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // e (101)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // f (102)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // g (103)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // h (104)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // i (105)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // j (106)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // k (107)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // l (108)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // m (109)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // n (110)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // o (111)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // p (112)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // q (113)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // r (114)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // s (115)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // t (116)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // u (117)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // v (118)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // w (119)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // x (120)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // y (121)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // z (122)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
    // { (123)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // | (124)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 4 },
    // } (125)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 6 },
    // ~ (126)
    CharMetrics { width: 8, height: 8, x_offset: 0, y_offset: 0, x_advance: 8 },
];

/// Packed bitmap data for 8x8 font (1 bit per pixel).
/// 
/// This data represents a simple 8x8 pixel font with basic ASCII characters.
/// Each character is 8x8 pixels, packed into bytes (8 pixels per byte).
/// The data is organized in a single row with all characters concatenated.
static FONT_8X8_DATA: [u8; 760] = [
    // Space (32) - 8x8 empty
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ! (33) - 8x8 exclamation
    0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00,
    // " (34) - 8x8 quote
    0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // # (35) - 8x8 hash
    0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00,
    // $ (36) - 8x8 dollar
    0x0C, 0x3F, 0x68, 0x3E, 0x0B, 0x7E, 0x18, 0x00,
    // % (37) - 8x8 percent
    0x60, 0x66, 0x0C, 0x18, 0x30, 0x66, 0x06, 0x00,
    // & (38) - 8x8 ampersand
    0x38, 0x6C, 0x6C, 0x38, 0x6D, 0x66, 0x3B, 0x00,
    // ' (39) - 8x8 apostrophe
    0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // ( (40) - 8x8 left paren
    0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00,
    // ) (41) - 8x8 right paren
    0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00,
    // * (42) - 8x8 asterisk
    0x00, 0x18, 0x7E, 0x3C, 0x7E, 0x18, 0x00, 0x00,
    // + (43) - 8x8 plus
    0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00,
    // , (44) - 8x8 comma
    0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30,
    // - (45) - 8x8 minus
    0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00,
    // . (46) - 8x8 period
    0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00,
    // / (47) - 8x8 slash
    0x00, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x00, 0x00,
    // 0 (48) - 8x8 zero
    0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00,
    // 1 (49) - 8x8 one
    0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00,
    // 2 (50) - 8x8 two
    0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00,
    // 3 (51) - 8x8 three
    0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00,
    // 4 (52) - 8x8 four
    0x06, 0x0E, 0x1E, 0x66, 0x7F, 0x06, 0x06, 0x00,
    // 5 (53) - 8x8 five
    0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00,
    // 6 (54) - 8x8 six
    0x3C, 0x66, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00,
    // 7 (55) - 8x8 seven
    0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00,
    // 8 (56) - 8x8 eight
    0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00,
    // 9 (57) - 8x8 nine
    0x3C, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00,
    // : (58) - 8x8 colon
    0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00,
    // ; (59) - 8x8 semicolon
    0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x30,
    // < (60) - 8x8 less than
    0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00,
    // = (61) - 8x8 equals
    0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00,
    // > (62) - 8x8 greater than
    0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00,
    // ? (63) - 8x8 question
    0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00,
    // @ (64) - 8x8 at
    0x3C, 0x66, 0x6E, 0x6A, 0x6E, 0x60, 0x3C, 0x00,
    // A (65) - 8x8 A
    0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00,
    // B (66) - 8x8 B
    0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00,
    // C (67) - 8x8 C
    0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00,
    // D (68) - 8x8 D
    0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00,
    // E (69) - 8x8 E
    0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00,
    // F (70) - 8x8 F
    0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00,
    // G (71) - 8x8 G
    0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3C, 0x00,
    // H (72) - 8x8 H
    0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00,
    // I (73) - 8x8 I
    0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00,
    // J (74) - 8x8 J
    0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00,
    // K (75) - 8x8 K
    0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00,
    // L (76) - 8x8 L
    0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00,
    // M (77) - 8x8 M
    0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00,
    // N (78) - 8x8 N
    0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00,
    // O (79) - 8x8 O
    0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00,
    // P (80) - 8x8 P
    0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00,
    // Q (81) - 8x8 Q
    0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00,
    // R (82) - 8x8 R
    0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00,
    // S (83) - 8x8 S
    0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00,
    // T (84) - 8x8 T
    0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00,
    // U (85) - 8x8 U
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00,
    // V (86) - 8x8 V
    0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00,
    // W (87) - 8x8 W
    0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00,
    // X (88) - 8x8 X
    0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00,
    // Y (89) - 8x8 Y
    0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00,
    // Z (90) - 8x8 Z
    0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00,
    // [ (91) - 8x8 left bracket
    0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00,
    // \ (92) - 8x8 backslash
    0x00, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x00, 0x00,
    // ] (93) - 8x8 right bracket
    0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00,
    // ^ (94) - 8x8 caret
    0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00,
    // _ (95) - 8x8 underscore
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x00,
    // ` (96) - 8x8 backtick
    0x30, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // a (97) - 8x8 a
    0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00,
    // b (98) - 8x8 b
    0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00,
    // c (99) - 8x8 c
    0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00,
    // d (100) - 8x8 d
    0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00,
    // e (101) - 8x8 e
    0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00,
    // f (102) - 8x8 f
    0x1C, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x30, 0x00,
    // g (103) - 8x8 g
    0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x3C,
    // h (104) - 8x8 h
    0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00,
    // i (105) - 8x8 i
    0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00,
    // j (106) - 8x8 j
    0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38,
    // k (107) - 8x8 k
    0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00,
    // l (108) - 8x8 l
    0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00,
    // m (109) - 8x8 m
    0x00, 0x00, 0x66, 0x7F, 0x7F, 0x6B, 0x63, 0x00,
    // n (110) - 8x8 n
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00,
    // o (111) - 8x8 o
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00,
    // p (112) - 8x8 p
    0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60,
    // q (113) - 8x8 q
    0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06,
    // r (114) - 8x8 r
    0x00, 0x00, 0x7C, 0x66, 0x60, 0x60, 0x60, 0x00,
    // s (115) - 8x8 s
    0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00,
    // t (116) - 8x8 t
    0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x1C, 0x00,
    // u (117) - 8x8 u
    0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00,
    // v (118) - 8x8 v
    0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00,
    // w (119) - 8x8 w
    0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00,
    // x (120) - 8x8 x
    0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00,
    // y (121) - 8x8 y
    0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x3C,
    // z (122) - 8x8 z
    0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00,
    // { (123) - 8x8 left brace
    0x0C, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0C, 0x00,
    // | (124) - 8x8 pipe
    0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00,
    // } (125) - 8x8 right brace
    0x30, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x30, 0x00,
    // ~ (126) - 8x8 tilde
    0x31, 0x6B, 0x46, 0x00, 0x00, 0x00, 0x00, 0x00,
];
