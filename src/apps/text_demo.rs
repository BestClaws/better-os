//! Time-based text showcase demonstrating font sizes and charsets.

use alloc::format;

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::font::{font_for_size, Charset, FontSize, MonoFont, DEFAULT_CHARSETS};
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Shape, Text};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use embassy_time::{Duration, Instant, Timer};

const STAGE_DURATION_MS: u64 = 5_000;

#[embassy_executor::task]
pub async fn text_demo_app(ctx: AppContext) {
    let start = Instant::now();
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let elapsed = Instant::now() - start;
        let stage_count = DEMO_STAGES.len();
        let stage_span = STAGE_DURATION_MS;
        let total_ms = elapsed.as_millis();
        let stage_index = ((total_ms / stage_span) % stage_count as u64) as usize;
        let stage_elapsed = (total_ms % stage_span) as f32;
        let stage_progress = (stage_elapsed / stage_span as f32).clamp(0.0, 1.0);

        ctx.draw(|surface: &mut DrawingSurface| {
            draw_stage(surface, stage_index, stage_progress);
        })
        .await;

        Timer::after(Duration::from_millis(60)).await;
    }
}

fn draw_stage(surface: &mut DrawingSurface, stage_index: usize, stage_progress: f32) {
    let stage = &DEMO_STAGES[stage_index];
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    surface.fill_rect(0, 0, width, height, stage.background);

    let margin = 14;
    let content_alpha = stage_alpha(stage_progress);
    let header_font = font_for_size(FontSize::Medium).with_charsets(DEFAULT_CHARSETS);
    let subtitle_font = font_for_size(FontSize::Small)
        .with_charsets(DEFAULT_CHARSETS)
        .with_letter_spacing(1);
    let label_font = font_for_size(FontSize::Small)
        .with_charsets(DEFAULT_CHARSETS)
        .with_letter_spacing(1);

    let mut cursor_y = margin;
    Text::new(margin, cursor_y, stage.title)
        .font(header_font)
        .color(Rgba8888::rgba(240, 240, 240, 255))
        .alpha(content_alpha)
        .draw(surface);
    cursor_y += header_font.line_advance() + 2;

    Text::new(margin, cursor_y, stage.subtitle)
        .font(subtitle_font)
        .color(Rgba8888::rgba(170, 190, 210, 230))
        .alpha(content_alpha)
        .draw(surface);
    cursor_y += subtitle_font.line_advance() + 10;

    let small_font = font_for_size(FontSize::Small).with_charsets(stage.charsets);
    let medium_font = font_for_size(FontSize::Medium).with_charsets(stage.charsets);
    let large_font = font_for_size(FontSize::Large).with_charsets(stage.charsets);

    cursor_y = draw_text_block(
        surface,
        "Small",
        stage.small_lines,
        small_font,
        label_font,
        margin,
        cursor_y,
        stage.accent,
        content_alpha,
    );
    cursor_y += 4;

    cursor_y = draw_text_block(
        surface,
        "Medium",
        stage.medium_lines,
        medium_font,
        label_font,
        margin,
        cursor_y,
        stage.accent,
        content_alpha,
    );
    cursor_y += 6;

    draw_text_block(
        surface,
        "Large",
        stage.large_lines,
        large_font,
        label_font,
        margin,
        cursor_y,
        stage.accent,
        content_alpha,
    );

    draw_progress(surface, stage_index, stage_progress, label_font, stage);
}

fn draw_text_block(
    surface: &mut DrawingSurface,
    label: &str,
    lines: &[&str],
    font: MonoFont,
    label_font: MonoFont,
    x: i32,
    mut y: i32,
    color: Rgba8888,
    alpha: u8,
) -> i32 {
    Text::new(x, y, label)
        .font(label_font)
        .color(Rgba8888::rgba(200, 205, 220, 240))
        .alpha(alpha)
        .draw(surface);
    y += label_font.line_advance();

    for line in lines {
        Text::new(x, y, line)
            .font(font)
            .color(color)
            .alpha(alpha)
            .draw(surface);
        y += font.line_advance();
    }

    y
}

fn draw_progress(
    surface: &mut DrawingSurface,
    stage_index: usize,
    stage_progress: f32,
    label_font: MonoFont,
    stage: &DemoStage,
) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    let margin = 14;
    let bar_height = 4;
    let bar_width = (width - 2 * margin).max(0);
    let bar_x = margin;
    let bar_y = height - margin - bar_height;

    if bar_width <= 0 || bar_y < 0 {
        return;
    }

    surface.fill_rect(bar_x, bar_y, bar_width, bar_height, Rgba8888::rgba(28, 32, 42, 220));
    let filled = ((stage_progress.clamp(0.0, 1.0) * bar_width as f32) + 0.5) as i32;
    let filled = filled.clamp(0, bar_width);
    if filled > 0 {
        surface.fill_rect(bar_x, bar_y, filled, bar_height, stage.accent);
    }

    let info_y = (bar_y - label_font.line_advance() - 6).max(margin);
    let info_text = format!(
        "Stage {}/{} • {}",
        stage_index + 1,
        DEMO_STAGES.len(),
        stage.title
    );

    Text::new(bar_x, info_y, info_text.as_str())
        .font(label_font)
        .color(Rgba8888::rgba(190, 200, 215, 230))
        .alpha(stage_alpha(stage_progress))
        .draw(surface);
}

fn stage_alpha(progress: f32) -> u8 {
    let fade_in = (progress * 2.0).min(1.0);
    let fade_out = ((1.0 - progress) * 2.0).min(1.0);
    let fade = fade_in.min(fade_out);
    (fade * 255.0) as u8
}

type Lines = &'static [&'static str];

struct DemoStage {
    title: &'static str,
    subtitle: &'static str,
    small_lines: Lines,
    medium_lines: Lines,
    large_lines: Lines,
    charsets: &'static [Charset],
    accent: Rgba8888,
    background: Rgba8888,
}

const CHARSETS_LATIN: &[Charset] = &[Charset::Basic, Charset::Latin];
const CHARSETS_GREEK: &[Charset] = &[Charset::Basic, Charset::Greek];
const CHARSETS_BOX: &[Charset] = &[Charset::Basic, Charset::BoxDrawing, Charset::Block];
const CHARSETS_MISC: &[Charset] = &[Charset::Basic, Charset::Misc];
const CHARSETS_HIRAGANA: &[Charset] = &[Charset::Basic, Charset::Hiragana];
const CHARSETS_SGA: &[Charset] = &[Charset::Basic, Charset::Sga];

const LATIN_SMALL: Lines = &[
    "The quick brown fox jumps over the lazy dog",
    "Sphinx of black quartz, judge my vow",
];
const LATIN_MEDIUM: Lines = &["Text rendering demo"];
const LATIN_LARGE: Lines = &["Better-OS"];

const GREEK_SMALL: Lines = &["Αλφα Βήτα Γάμμα Δέλτα", "Θήτα Ιώτα Κάππα Λάμδα"];
const GREEK_MEDIUM: Lines = &["Μικρό δείγμα"];
const GREEK_LARGE: Lines = &["Ωμέγα"];

const BOX_SMALL: Lines = &["┌─┬─┐ ┏━┳━┓", "│ │ │ ┃ ┃ ┃"];
const BOX_MEDIUM: Lines = &["├─┼─┤ ┣━╋━┫"];
const BOX_LARGE: Lines = &["└─┴─┘ ┻━┻"];

const MISC_SMALL: Lines = &["★ ☆ ☀ ☁", "♪ ♫ ☂ ☕"];
const MISC_MEDIUM: Lines = &["← ↑ → ↓"];
const MISC_LARGE: Lines = &["✓✗"];

const HIRAGANA_SMALL: Lines = &["こんにちは", "おはよう"];
const HIRAGANA_MEDIUM: Lines = &["さようなら ありがとう"];
const HIRAGANA_LARGE: Lines = &["またね"];

const SGA_SMALL: Lines = &["𐑐𐑑𐑒𐑓", "𐑕𐑰 𐑞 𐑣𐑵𐑑"];
const SGA_MEDIUM: Lines = &["𐑕𐑒𐑱𐑚𐑰𐑛 𐑛𐑧𐑥𐑴"];
const SGA_LARGE: Lines = &["𐑒𐑨𐑦𐑮𐑦"];

const DEMO_STAGES: [DemoStage; 6] = [
    DemoStage {
        title: "Latin & ASCII",
        subtitle: "Charsets: Basic + Latin",
        small_lines: LATIN_SMALL,
        medium_lines: LATIN_MEDIUM,
        large_lines: LATIN_LARGE,
        charsets: CHARSETS_LATIN,
        accent: Rgba8888::rgba(255, 196, 112, 255),
        background: Rgba8888::rgba(24, 28, 37, 255),
    },
    DemoStage {
        title: "Greek",
        subtitle: "Charsets: Basic + Greek",
        small_lines: GREEK_SMALL,
        medium_lines: GREEK_MEDIUM,
        large_lines: GREEK_LARGE,
        charsets: CHARSETS_GREEK,
        accent: Rgba8888::rgba(140, 191, 255, 255),
        background: Rgba8888::rgba(22, 28, 36, 255),
    },
    DemoStage {
        title: "Box & Block",
        subtitle: "Charsets: Basic + BoxDrawing + Block",
        small_lines: BOX_SMALL,
        medium_lines: BOX_MEDIUM,
        large_lines: BOX_LARGE,
        charsets: CHARSETS_BOX,
        accent: Rgba8888::rgba(255, 155, 196, 255),
        background: Rgba8888::rgba(20, 24, 34, 255),
    },
    DemoStage {
        title: "Misc Symbols",
        subtitle: "Charsets: Basic + Misc",
        small_lines: MISC_SMALL,
        medium_lines: MISC_MEDIUM,
        large_lines: MISC_LARGE,
        charsets: CHARSETS_MISC,
        accent: Rgba8888::rgba(173, 255, 194, 255),
        background: Rgba8888::rgba(18, 27, 30, 255),
    },
    DemoStage {
        title: "Hiragana",
        subtitle: "Charsets: Basic + Hiragana",
        small_lines: HIRAGANA_SMALL,
        medium_lines: HIRAGANA_MEDIUM,
        large_lines: HIRAGANA_LARGE,
        charsets: CHARSETS_HIRAGANA,
        accent: Rgba8888::rgba(255, 210, 130, 255),
        background: Rgba8888::rgba(24, 24, 36, 255),
    },
    DemoStage {
        title: "Shavian",
        subtitle: "Charsets: Basic + SGA",
        small_lines: SGA_SMALL,
        medium_lines: SGA_MEDIUM,
        large_lines: SGA_LARGE,
        charsets: CHARSETS_SGA,
        accent: Rgba8888::rgba(210, 180, 255, 255),
        background: Rgba8888::rgba(26, 22, 34, 255),
    },
];
