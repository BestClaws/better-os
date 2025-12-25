# Graphics Font Modules

- `src/libs/gfx/font.rs` wraps the public-domain `font8x8` unicode tables in a `MonoFont` abstraction. Glyphs are fetched directly from the bundled character sets (Basic Latin, Latin-1, Greek, box-drawing, block elements, misc symbols, Hiragana, and the SGA set) and scaled on the fly, so we get wide Unicode coverage without copying upstream sources.
- `GlyphBitmap` exposes per-glyph helpers (`width`, `height`, `is_pixel_inked`) that honour the active scale factor while sampling straight from the 8×8 masks.
- `src/libs/gfx/shapes/text.rs` owns the layout logic. It blends glyphs into any `Rasterizer`, honours per-instance colour/alpha, supports line breaks, and lets call sites pick fonts via `.font(MonoFont)`, `.size(FontSize)`, `.charsets(&[Charset])`, or `.letter_spacing(i8)`.
- `docs/gfx-fonts.md` (this file) captures how those pieces cooperate so new contributors know where glyph data lives and how to extend it.

## Default Options

- `font_for_size(FontSize::Small | Medium | Large)` returns the stock monospace set at 1×, 2×, or 3× scale respectively.
- `DEFAULT_CHARSETS` lists the lookup order; pass a custom slice to `MonoFont::with_charsets` if you want to restrict or extend coverage.
- `Charset` enumerates every bundled table (Basic, Latin, Greek, Block, BoxDrawing, Misc, Hiragana, Sga).

Combine them to suit each widget: `Text::new(...).size(FontSize::Medium).charsets(&[Charset::Basic, Charset::Greek])` renders a medium-sized Latin/Greek overlay, while `.font(MonoFont::for_size_with_charsets(FontSize::Large, DEFAULT_CHARSETS).with_letter_spacing(0))` gives you large glyphs with tight spacing.
