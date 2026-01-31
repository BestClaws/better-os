#!/usr/bin/env python3
"""Generate Rust font modules from LVGL Montserrat font C sources."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

ROOT = Path(__file__).resolve().parents[2]
LVGL_FONT_DIR = ROOT / "lvgl" / "src" / "font"
OUT_DIR = (
    ROOT
    / "rust-gfx"
    / "src"
    / "primitives"
    / "label"
    / "label_font"
    / "fonts"
)

FONT_SPECS: Sequence[Tuple[str, str]] = (
    ("Montserrat8", "lv_font_montserrat_8.c"),
    ("Montserrat10", "lv_font_montserrat_10.c"),
    ("Montserrat12", "lv_font_montserrat_12.c"),
    ("Montserrat14", "lv_font_montserrat_14.c"),
    ("Montserrat16", "lv_font_montserrat_16.c"),
    ("Montserrat18", "lv_font_montserrat_18.c"),
    ("Montserrat20", "lv_font_montserrat_20.c"),
    ("Montserrat22", "lv_font_montserrat_22.c"),
    ("Montserrat24", "lv_font_montserrat_24.c"),
    ("Montserrat26", "lv_font_montserrat_26.c"),
    ("Montserrat28", "lv_font_montserrat_28.c"),
    ("Montserrat30", "lv_font_montserrat_30.c"),
    ("Montserrat32", "lv_font_montserrat_32.c"),
    ("Montserrat34", "lv_font_montserrat_34.c"),
    ("Montserrat36", "lv_font_montserrat_36.c"),
    ("Montserrat38", "lv_font_montserrat_38.c"),
    ("Montserrat40", "lv_font_montserrat_40.c"),
    ("Montserrat42", "lv_font_montserrat_42.c"),
    ("Montserrat44", "lv_font_montserrat_44.c"),
    ("Montserrat46", "lv_font_montserrat_46.c"),
    ("Montserrat48", "lv_font_montserrat_48.c"),
)


@dataclass
class GlyphEntry:
    adv_w: int
    box_w: int
    box_h: int
    ofs_x: int
    ofs_y: int
    bitmap_index: int


@dataclass
class CMapEntry:
    range_start: int
    range_length: int
    glyph_id_start: int
    unicode_list: Optional[str]
    list_length: int
    cmap_type: str


@dataclass
class FontMeta:
    line_height: int
    base_line: int
    underline_position: int
    underline_thickness: int
    bpp: int
    kern_scale: int
    kerning_enabled: bool
    left_class_cnt: Optional[int]
    right_class_cnt: Optional[int]


@dataclass
class ParsedFont:
    variant: str
    symbol: str
    glyph_bitmap_bytes: List[int]
    glyphs: List[GlyphEntry]
    unicode_lists: Dict[str, List[int]]
    cmaps: List[CMapEntry]
    kern_left: List[int]
    kern_right: List[int]
    kern_values: List[int]
    meta: FontMeta


def extract_braced_block(text: str, marker: str, *, reverse: bool = False) -> str:
    idx = text.rfind(marker) if reverse else text.find(marker)
    if idx == -1:
        raise ValueError(f"Marker '{marker}' not found")
    start = text.find("{", idx)
    if start == -1:
        raise ValueError(f"Opening brace for marker '{marker}' not found")
    depth = 1
    i = start + 1
    while depth > 0 and i < len(text):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        i += 1
    if depth != 0:
        raise ValueError(f"Unbalanced braces after marker '{marker}'")
    return text[start + 1 : i - 1]


def strip_comments(text: str) -> str:
    return re.sub(r"/\*.*?\*/", "", text, flags=re.S)


def parse_numbers(block: str) -> List[int]:
    block = strip_comments(block)
    tokens = re.findall(r"[-+]?0x[0-9a-fA-F]+|[-+]?\d+", block)
    return [int(tok, 0) for tok in tokens]


def split_top_level_entries(block: str) -> List[str]:
    entries: List[str] = []
    i = 0
    length = len(block)
    while i < length:
        while i < length and block[i] != "{":
            i += 1
        if i >= length:
            break
        depth = 1
        start = i + 1
        i += 1
        while i < length and depth > 0:
            if block[i] == "{":
                depth += 1
            elif block[i] == "}":
                depth -= 1
            i += 1
        if depth != 0:
            raise ValueError("Unbalanced braces in entry")
        entries.append(block[start : i - 1])
    return entries


def parse_glyphs(block: str) -> List[GlyphEntry]:
    entries = split_top_level_entries(block)
    glyphs: List[GlyphEntry] = []
    for entry in entries:
        entry_nc = strip_comments(entry)
        props = dict(re.findall(r"\.(\w+)\s*=\s*([-+]?\d+)", entry_nc))
        glyphs.append(
            GlyphEntry(
                adv_w=int(props["adv_w"]),
                box_w=int(props["box_w"]),
                box_h=int(props["box_h"]),
                ofs_x=int(props["ofs_x"]),
                ofs_y=int(props["ofs_y"]),
                bitmap_index=int(props["bitmap_index"]),
            )
        )
    return glyphs


def parse_cmaps(block: str) -> List[CMapEntry]:
    entries = split_top_level_entries(block)
    cmaps: List[CMapEntry] = []
    for entry in entries:
        entry_nc = strip_comments(entry)
        props = dict(
            (k, v.strip())
            for k, v in re.findall(r"\.(\w+)\s*=\s*([^,]+)", entry_nc)
        )
        cmaps.append(
            CMapEntry(
                range_start=int(props["range_start"], 0),
                range_length=int(props["range_length"], 0),
                glyph_id_start=int(props["glyph_id_start"], 0),
                unicode_list=None if props["unicode_list"] == "NULL" else props["unicode_list"],
                list_length=int(props["list_length"], 0),
                cmap_type=props["type"],
            )
        )
    return cmaps


def parse_font_meta(text: str, symbol: str) -> FontMeta:
    font_dsc_block = strip_comments(extract_braced_block(text, "font_dsc =", reverse=True))
    font_props = dict(
        (k, v.strip())
        for k, v in re.findall(r"\.(\w+)\s*=\s*([^,]+)", font_dsc_block)
    )
    kern_dsc = font_props.get("kern_dsc", "NULL")
    kerning_enabled = kern_dsc != "NULL"
    kern_scale = int(font_props.get("kern_scale", "16"), 0)
    bpp = int(font_props.get("bpp", "4"), 0)

    left_class_cnt = right_class_cnt = None
    if kerning_enabled:
        kern_classes_block = strip_comments(
            extract_braced_block(text, "kern_classes =")
        )
        kern_props = dict(
            (k, v.strip())
            for k, v in re.findall(r"\.(\w+)\s*=\s*([^,]+)", kern_classes_block)
        )
        left_class_cnt = int(kern_props.get("left_class_cnt", "0"), 0)
        right_class_cnt = int(kern_props.get("right_class_cnt", "0"), 0)

    font_struct_block = strip_comments(
        extract_braced_block(text, f"{symbol} =", reverse=True)
    )
    struct_props = dict(
        (k, v.strip())
        for k, v in re.findall(r"\.(\w+)\s*=\s*([^,]+)", font_struct_block)
    )
    line_height = int(struct_props.get("line_height", "0"), 0)
    base_line = int(struct_props.get("base_line", "0"), 0)
    underline_position = int(struct_props.get("underline_position", "0"), 0)
    underline_thickness = int(struct_props.get("underline_thickness", "0"), 0)

    return FontMeta(
        line_height=line_height,
        base_line=base_line,
        underline_position=underline_position,
        underline_thickness=underline_thickness,
        bpp=bpp,
        kern_scale=kern_scale,
        kerning_enabled=kerning_enabled,
        left_class_cnt=left_class_cnt,
        right_class_cnt=right_class_cnt,
    )


def parse_font(c_path: Path, variant: str, symbol: str) -> ParsedFont:
    text = c_path.read_text()
    glyph_bitmap = parse_numbers(extract_braced_block(text, "glyph_bitmap[]"))
    glyphs = parse_glyphs(extract_braced_block(text, "glyph_dsc[]"))

    unicode_lists: Dict[str, List[int]] = {}
    for match in re.finditer(r"static const uint16_t (unicode_list_\w+)\[\] =", text):
        list_name = match.group(1)
        unicode_lists[list_name] = parse_numbers(
            extract_braced_block(text, f"{list_name}[]")
        )

    cmaps = parse_cmaps(extract_braced_block(text, "cmaps[]"))

    kern_left = parse_numbers(
        extract_braced_block(text, "kern_left_class_mapping[]")
    )
    kern_right = parse_numbers(
        extract_braced_block(text, "kern_right_class_mapping[]")
    )
    kern_values = parse_numbers(
        extract_braced_block(text, "kern_class_values[]")
    )

    meta = parse_font_meta(text, symbol)

    return ParsedFont(
        variant=variant,
        symbol=symbol,
        glyph_bitmap_bytes=glyph_bitmap,
        glyphs=glyphs,
        unicode_lists=unicode_lists,
        cmaps=cmaps,
        kern_left=kern_left,
        kern_right=kern_right,
        kern_values=kern_values,
        meta=meta,
    )


def decode_pixels(font: ParsedFont) -> Tuple[List[int], List[int], List[int]]:
    raw = font.glyph_bitmap_bytes
    bpp = font.meta.bpp
    if bpp <= 0:
        raise ValueError("Unsupported bpp")
    mask = (1 << bpp) - 1
    scaled_pixels: List[int] = []
    bitmap_offsets: List[int] = []
    bitmap_lengths: List[int] = []

    for glyph in font.glyphs:
        pixel_count = glyph.box_w * glyph.box_h
        bitmap_offsets.append(len(scaled_pixels))
        bitmap_lengths.append(pixel_count)
        if pixel_count == 0:
            continue
        start_bit = glyph.bitmap_index * 8
        values: List[int] = []
        for i in range(pixel_count):
            bit_pos = start_bit + i * bpp
            byte_idx = bit_pos // 8
            bit_offset = bit_pos % 8
            if byte_idx >= len(raw):
                values.append(0)
                continue
            hi = raw[byte_idx]
            lo = raw[byte_idx + 1] if (byte_idx + 1) < len(raw) else 0
            window = (hi << 8) | lo
            shift = 16 - bit_offset - bpp
            if shift < 0:
                shift = 0
            val = (window >> shift) & mask
            scaled = (val * 255 + mask // 2) // mask if mask else 0
            values.append(scaled)
        scaled_pixels.extend(values)

    return scaled_pixels, bitmap_offsets, bitmap_lengths


def build_glyph_map(font: ParsedFont) -> List[Tuple[int, int]]:
    mapping: List[Tuple[int, int]] = []
    unicode_lists = font.unicode_lists
    for cmap in font.cmaps:
        glyph_id = cmap.glyph_id_start
        if cmap.cmap_type == "LV_FONT_FMT_TXT_CMAP_FORMAT0_TINY":
            for offset in range(cmap.range_length):
                codepoint = cmap.range_start + offset
                mapping.append((codepoint, glyph_id + offset))
        elif cmap.cmap_type == "LV_FONT_FMT_TXT_CMAP_SPARSE_TINY":
            if not cmap.unicode_list:
                continue
            unicode_values = unicode_lists.get(cmap.unicode_list, [])
            for idx, delta in enumerate(unicode_values):
                codepoint = cmap.range_start + delta
                mapping.append((codepoint, glyph_id + idx))
        else:
            raise ValueError(f"Unsupported cmap type {cmap.cmap_type}")
    mapping.sort(key=lambda item: item[0])
    return mapping


def format_numbers(values: Sequence[int], indent: int = 4, per_line: int = 12) -> str:
    lines: List[str] = []
    line: List[str] = []
    for value in values:
        line.append(str(value))
        if len(line) >= per_line:
            lines.append(", ".join(line))
            line = []
    if line:
        lines.append(", ".join(line))
    indent_str = " " * indent
    return (",\n".join(f"{indent_str}{ln}" for ln in lines))


def format_chars(mapping: List[Tuple[int, int]], indent: int = 4) -> str:
    lines: List[str] = []
    indent_str = " " * indent
    for codepoint, glyph_idx in mapping:
        ch = chr(codepoint)
        if ch == "\\":
            char_repr = "'\\\\'"
        elif ch == "'":
            char_repr = "'\\''"
        elif 32 <= codepoint <= 126:
            char_repr = f"'{ch}'"
        else:
            char_repr = f"'\\u{{{codepoint:04X}}}'"
        lines.append(
            f"{indent_str}GlyphMap {{ ch: {char_repr}, glyph_index: {glyph_idx} }},"
        )
    return "\n".join(lines)


def render_module(font: ParsedFont) -> str:
    pixels, offsets, lengths = decode_pixels(font)
    mapping = build_glyph_map(font)

    kern_block = ""
    if font.meta.kerning_enabled:
        kern_block = f"\npub(super) static FONT_KERNING: FontKerning = FontKerning {{\n    left_class_mapping: &KERN_LEFT_CLASS_MAPPING,\n    right_class_mapping: &KERN_RIGHT_CLASS_MAPPING,\n    class_pair_values: &KERN_CLASS_VALUES,\n    left_class_cnt: {font.meta.left_class_cnt or 0},\n    right_class_cnt: {font.meta.right_class_cnt or 0},\n    scale: {font.meta.kern_scale},\n}};\n"

    kerning_ref = "Some(&FONT_KERNING)" if font.meta.kerning_enabled else "None"

    glyph_entries: List[str] = []
    for idx, glyph in enumerate(font.glyphs):
        glyph_entries.append(
            "    Glyph {\n"
            f"        adv_w_raw: {glyph.adv_w},\n"
            f"        box_w: {glyph.box_w},\n"
            f"        box_h: {glyph.box_h},\n"
            f"        ofs_x: {glyph.ofs_x},\n"
            f"        ofs_y: {glyph.ofs_y},\n"
            f"        bitmap_offset: {offsets[idx]},\n"
            f"        bitmap_len: {lengths[idx]},\n"
            "    },"
        )

    module = f"// Auto-generated from {font.symbol}\n" \
        "use super::*;\n\n" \
        f"pub(super) static FONT_BITMAP: [u8; {len(pixels)}] = [\n" \
        f"{format_numbers(pixels)}\n" \
        "];\n\n" \
        f"pub(super) static FONT_GLYPHS: [Glyph; {len(font.glyphs)}] = [\n" \
        f"{chr(10).join(glyph_entries)}\n" \
        "];\n\n" \
        f"pub(super) static FONT_GLYPH_MAP: [GlyphMap; {len(mapping)}] = [\n" \
        f"{format_chars(mapping)}\n" \
        "];\n\n"

    if font.meta.kerning_enabled:
        module += (
            f"pub(super) static KERN_LEFT_CLASS_MAPPING: [u8; {len(font.kern_left)}] = [\n"
            f"{format_numbers(font.kern_left)}\n" \
            "];\n\n"
            f"pub(super) static KERN_RIGHT_CLASS_MAPPING: [u8; {len(font.kern_right)}] = [\n"
            f"{format_numbers(font.kern_right)}\n" \
            "];\n\n"
            f"pub(super) static KERN_CLASS_VALUES: [i8; {len(font.kern_values)}] = [\n"
            f"{format_numbers(font.kern_values)}\n" \
            "];\n\n"
        )

    module += (
        kern_block
        + f"pub(super) static FONT: Font = Font {{\n"
        f"    id: FontId::{font.variant},\n"
        f"    name: \"{font.symbol}\",\n"
        f"    line_height: {font.meta.line_height},\n"
        f"    base_line: {font.meta.base_line},\n"
        f"    underline_position: {font.meta.underline_position},\n"
        f"    underline_thickness: {font.meta.underline_thickness},\n"
        "    glyphs: &FONT_GLYPHS,\n"
        "    glyph_map: &FONT_GLYPH_MAP,\n"
        "    bitmap: &FONT_BITMAP,\n"
        f"    kerning: {kerning_ref},\n"
        "};\n"
    )

    return module


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate Montserrat font modules")
    parser.add_argument(
        "--out-dir", type=Path, default=OUT_DIR, help="Output directory for Rust modules"
    )
    parser.add_argument(
        "--lvgl-dir", type=Path, default=LVGL_FONT_DIR, help="Directory containing LVGL fonts"
    )
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)

    for font_name, c_filename in FONT_SPECS:
        c_path = args.lvgl_dir / c_filename
        if not c_path.exists():
            raise FileNotFoundError(f"Missing LVGL font source: {c_path}")
        symbol = c_filename.split(".")[0]
        parsed = parse_font(c_path, font_name, symbol)
        module_code = render_module(parsed)
        module_name = font_name.lower().replace("montserrat", "montserrat_")
        module_path = args.out_dir / f"{module_name}.rs"
        module_path.write_text(module_code)
        print(f"Generated {module_path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
