use alloc::string::String;

use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::label::{draw_label, measure_text_with_font, FontId, LabelDsc};
use rust_gfx::primitives::rectangle::{draw_rect, RectDsc};
use rust_gfx::types::{Area, BorderSide, Gradient, OPA_COVER, RADIUS_CIRCLE};

use crate::system::ui::drawing_surface::DrawingSurface;

use super::theme::{StyleFonts, StyleMetrics, StylePalette};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentColor {
    Yellow,
    Gray,
    Dark,
    Custom(Rgba8888),
}

impl AccentColor {
    fn resolve(self, palette: StylePalette) -> Rgba8888 {
        match self {
            AccentColor::Yellow => palette.accent_yellow,
            AccentColor::Gray => palette.accent_gray,
            AccentColor::Dark => palette.accent_dark,
            AccentColor::Custom(color) => color,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeTone {
    Accent,
    Gray,
    Danger,
}

#[derive(Clone, Copy, Debug)]
pub struct BadgeConfig<'a> {
    pub text: &'a str,
    pub tone: BadgeTone,
}

#[derive(Clone, Copy, Debug)]
pub struct StatusBarData<'a> {
    pub left: &'a str,
    pub battery_percent: u8,
    pub right: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub struct TimeDisplayData<'a> {
    pub time_text: &'a str,
    pub date_text: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct CardConfig<'a> {
    pub title: Option<&'a str>,
    pub subtitle: Option<&'a str>,
    pub badge: Option<BadgeConfig<'a>>,
    pub accent: AccentColor,
    pub height: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct CardFrame {
    pub card_area: Area,
    pub content_area: Area,
}

impl CardFrame {
    pub fn next_y(&self, metrics: &StyleMetrics) -> i32 {
        self.card_area.y2 + 1 + metrics.section_spacing
    }

    pub fn content_origin(&self) -> (i32, i32) {
        (self.content_area.x1, self.content_area.y1)
    }

    pub fn content_width(&self) -> i32 {
        self.content_area.x2 - self.content_area.x1 + 1
    }
}

#[derive(Clone, Copy, Debug)]
pub struct QuickAction<'a> {
    pub icon: &'a str,
    pub label: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct StatCardConfig<'a> {
    pub value: &'a str,
    pub label: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct ProgressBarConfig<'a> {
    pub percent: u8,
    pub label: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub struct ToggleConfig<'a> {
    pub label: &'a str,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Icon,
    IconSecondary,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonConfig<'a> {
    pub text: &'a str,
    pub kind: ButtonKind,
}

pub fn draw_background(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    palette: StylePalette,
) {
    let width = metrics.width.max(0);
    let height = metrics.height.max(0);
    if width == 0 || height == 0 {
        return;
    }

    let mut base = RectDsc::new();
    base.bg_color = palette.background;
    base.bg_opa = OPA_COVER;
    let base_area = Area::new(0, 0, width - 1, height - 1);
    draw_rect(surface, &base, &base_area);

    let inset = metrics.outer_padding / 2;
    if inset >= width / 2 || inset >= height / 2 {
        return;
    }

    // let container_area = Area::new(inset, inset, width - inset - 1, height - inset - 1);
    // let mut container = RectDsc::new();
    // container.bg_color = palette.container;
    // container.bg_grad = Gradient::vertical(palette.container, palette.container_alt);
    // container.bg_opa = OPA_COVER;
    // container.radius = metrics.section_radius + 2;
    // container.border_width = 1;
    // container.border_color = palette.outline;
    // container.border_opa = OPA_COVER;
    // draw_rect(surface, &container, &container_area);
}

pub fn draw_status_bar(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    data: StatusBarData,
) -> Area {
    let height = fonts.line_height_status() + metrics.section_padding * 2;
    let top = metrics.outer_padding;
    let left = metrics.content_x();
    let width = metrics.content_width();

    let area = Area::new(left, top, left + width - 1, top + height - 1);

    let mut bar = RectDsc::new();
    bar.bg_color = palette.accent_dark;
    bar.bg_grad = Gradient::vertical(palette.accent_dark, Rgba8888::rgba(34, 34, 34, 255));
    bar.bg_opa = OPA_COVER;
    bar.radius = metrics.section_radius.max(2);
    draw_rect(surface, &bar, &area);

    let text_y = top + metrics.section_padding;
    let mut cursor_x = left + metrics.section_padding;
    let left_width = draw_text(
        surface,
        data.left,
        fonts.status,
        cursor_x,
        text_y,
        palette.container,
    );
    cursor_x += left_width + metrics.section_padding;

    if let Some(right) = data.right {
        let right_width = measure_text_with_font(right, 0, fonts.status);
        let right_x = (area.x2 - metrics.section_padding - right_width).max(cursor_x);
        let _ = draw_text(
            surface,
            right,
            fonts.status,
            right_x,
            text_y,
            palette.container,
        );
    }

    draw_battery(
        surface,
        &area,
        metrics,
        fonts,
        palette,
        data.battery_percent,
    );

    area
}

pub fn draw_time_display(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    data: TimeDisplayData,
    top: i32,
) -> Area {
    let width = metrics.content_width();
    let left = metrics.content_x();
    let display_height = fonts.line_height_display();
    let date_height = fonts.line_height_body();
    let total_height = display_height + date_height + metrics.section_padding * 2 + 4;

    let area = Area::new(left, top, left + width - 1, top + total_height - 1);

    let mut block = RectDsc::new();
    block.bg_color = palette.container;
    block.bg_grad = Gradient::vertical(palette.container, palette.container_alt);
    block.bg_opa = OPA_COVER;
    block.radius = metrics.section_radius.max(3);
    draw_rect(surface, &block, &area);

    let time_width = measure_text_with_font(data.time_text, 0, fonts.display);
    let time_x = left + (width - time_width) / 2;
    let time_y = top + metrics.section_padding;
    draw_text(
        surface,
        data.time_text,
        fonts.display,
        time_x,
        time_y,
        palette.text_primary,
    );

    let date_width = measure_text_with_font(data.date_text, 0, fonts.body);
    let date_x = left + (width - date_width) / 2;
    let date_y = area.y2 - metrics.section_padding - date_height;
    draw_text(
        surface,
        data.date_text,
        fonts.body,
        date_x,
        date_y,
        palette.text_secondary,
    );

    // Return area for chaining.
    area
}

pub fn draw_card(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    top: i32,
    config: CardConfig,
) -> CardFrame {
    let min_height = metrics.section_padding * 2 + fonts.line_height_body();
    let height = config.height.max(min_height);
    let bottom = (top + height - 1).min(metrics.height - 1);
    let card_area = Area::new(
        metrics.content_x(),
        top,
        metrics.content_x() + metrics.content_width() - 1,
        bottom,
    );

    let accent_color = config.accent.resolve(palette);

    let mut shadow = RectDsc::new();
    shadow.bg_color = palette.shadow;
    shadow.bg_opa = OPA_COVER;
    shadow.radius = metrics.section_radius;
    let shadow_area = Area::new(
        card_area.x1 + 2,
        card_area.y1 + 3,
        card_area.x2 + 2,
        card_area.y2 + 3,
    );
    draw_rect(surface, &shadow, &shadow_area);

    let mut card = RectDsc::new();
    card.bg_color = palette.container;
    card.bg_grad = Gradient::vertical(palette.container, palette.container_alt);
    card.bg_opa = OPA_COVER;
    card.radius = metrics.section_radius;
    card.border_width = 1;
    card.border_color = palette.outline;
    card.border_opa = OPA_COVER;
    draw_rect(surface, &card, &card_area);

    let accent_width = 4;
    let mut accent = RectDsc::new();
    accent.bg_opa = 0;
    accent.border_width = accent_width;
    accent.border_color = accent_color;
    accent.border_opa = OPA_COVER;
    accent.border_side = BorderSide::LEFT;
    accent.radius = metrics.section_radius.max(3);
    draw_rect(surface, &accent, &card_area);

    let mut content_x = card_area.x1 + metrics.section_padding + accent_width;
    let content_right = card_area.x2 - metrics.section_padding;
    if content_x > content_right {
        content_x = card_area.x1 + metrics.section_padding;
    }
    let mut content_y = card_area.y1 + metrics.section_padding;

    if let Some(title) = config.title {
        draw_text(
            surface,
            title,
            fonts.title,
            content_x,
            content_y,
            palette.text_primary,
        );
        if let Some(subtitle) = config.subtitle {
            let sub_y = content_y + fonts.line_height_title() + 2;
            draw_text(
                surface,
                subtitle,
                fonts.body,
                content_x,
                sub_y,
                palette.text_secondary,
            );
            content_y = sub_y + fonts.line_height_body() + 4;
        } else {
            content_y += fonts.line_height_title() + 4;
        }

        if let Some(badge) = config.badge {
            draw_badge(surface, fonts, palette, badge, content_right, content_y);
        }
    }

    let content_area = Area::new(
        content_x,
        content_y,
        content_right,
        card_area.y2 - metrics.section_padding,
    );

    CardFrame {
        card_area,
        content_area,
    }
}

pub fn draw_quick_actions(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    actions: &[QuickAction],
) {
    if actions.is_empty() {
        return;
    }
    let columns = 2;
    let rows = (actions.len() as i32 + columns - 1) / columns;
    if rows <= 0 {
        return;
    }
    let cell_width = (frame.content_width() - metrics.quick_gap) / columns;
    let cell_height = metrics.button_height;

    for (idx, action) in actions.iter().enumerate() {
        let row = idx as i32 / columns;
        let col = idx as i32 % columns;
        let x = frame.content_area.x1 + col * (cell_width + metrics.quick_gap);
        let y = frame.content_area.y1 + row * (cell_height + metrics.quick_gap);

        let tile_area = Area::new(x, y, x + cell_width - 1, y + cell_height - 1);
        let mut tile = RectDsc::new();
        tile.bg_color = palette.container_alt;
        tile.bg_opa = OPA_COVER;
        tile.border_width = 1;
        tile.border_color = palette.outline;
        tile.border_opa = OPA_COVER;
        tile.radius = metrics.section_radius;
        draw_rect(surface, &tile, &tile_area);

        let icon_width = measure_text_with_font(action.icon, 0, fonts.display);
        let icon_x = x + (cell_width - icon_width) / 2;
        let icon_y = y + 2;
        draw_text(
            surface,
            action.icon,
            fonts.display,
            icon_x,
            icon_y,
            palette.text_primary,
        );

        let label_width = measure_text_with_font(action.label, 0, fonts.body);
        let label_x = x + (cell_width - label_width) / 2;
        let label_y = tile_area.y2 - fonts.line_height_body() - 2;
        draw_text(
            surface,
            action.label,
            fonts.body,
            label_x,
            label_y,
            palette.text_secondary,
        );
    }
}

pub fn draw_stat_card(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    stats: &[StatCardConfig],
) {
    if stats.is_empty() {
        return;
    }

    let columns = stats.len().min(2) as i32;
    let cell_width = (frame.content_width() - metrics.quick_gap * (columns - 1)) / columns;
    let cell_height = metrics.button_height;

    for (idx, stat) in stats.iter().enumerate() {
        let column = idx as i32 % columns;
        let row = idx as i32 / columns;
        let x = frame.content_area.x1 + column * (cell_width + metrics.quick_gap);
        let y = frame.content_area.y1 + row * (cell_height + metrics.quick_gap);
        let area = Area::new(x, y, x + cell_width - 1, y + cell_height - 1);

        let mut card = RectDsc::new();
        card.bg_color = palette.stat_gradient_start;
        card.bg_grad = Gradient::vertical(palette.stat_gradient_start, palette.stat_gradient_end);
        card.bg_opa = OPA_COVER;
        card.radius = metrics.section_radius;
        draw_rect(surface, &card, &area);

        let value_width = measure_text_with_font(stat.value, 0, fonts.title);
        let value_x = x + 6;
        let value_y = y + 6;
        draw_text(
            surface,
            stat.value,
            fonts.title,
            value_x,
            value_y,
            palette.container,
        );

        let label_width = measure_text_with_font(stat.label, 0, fonts.small);
        let label_x = x + 6;
        let label_y = area.y2 - fonts.line_height_small() - 4;
        draw_text(
            surface,
            stat.label,
            fonts.small,
            label_x,
            label_y,
            palette.container,
        );
    }
}

pub fn draw_progress_bar(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    config: ProgressBarConfig,
) {
    let label_height = fonts.line_height_small();
    let bar_height = metrics.section_padding + 2;
    let bar_y = frame.content_area.y2 - bar_height;
    let label_y = bar_y - label_height - 2;

    if label_y >= frame.content_area.y1 {
        draw_text(
            surface,
            config.label,
            fonts.small,
            frame.content_area.x1,
            label_y,
            palette.text_secondary,
        );
    }

    let bar_area = Area::new(
        frame.content_area.x1,
        bar_y,
        frame.content_area.x2,
        bar_y + bar_height - 1,
    );

    let mut track = RectDsc::new();
    track.bg_color = palette.text_secondary;
    track.bg_opa = OPA_COVER;
    track.radius = bar_height / 2;
    draw_rect(surface, &track, &bar_area);

    if config.percent > 0 {
        let fill_width =
            ((bar_area.x2 - bar_area.x1 + 1) as u32 * config.percent as u32 / 100) as i32;
        if fill_width > 0 {
            let fill_area = Area::new(
                bar_area.x1,
                bar_area.y1,
                bar_area.x1 + fill_width - 1,
                bar_area.y2,
            );
            let mut fill = RectDsc::new();
            fill.bg_color = palette.accent_yellow;
            fill.bg_opa = OPA_COVER;
            fill.radius = bar_height / 2;
            draw_rect(surface, &fill, &fill_area);
        }
    }
}

pub fn draw_toggle(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    toggles: &[ToggleConfig],
) {
    if toggles.is_empty() {
        return;
    }

    let mut cursor_y = frame.content_area.y1;
    for toggle in toggles {
        draw_text(
            surface,
            toggle.label,
            fonts.body,
            frame.content_area.x1,
            cursor_y,
            palette.text_primary,
        );

        let switch_width = metrics.section_padding * 3;
        let switch_height = (fonts.line_height_body() * 3) / 4;
        let switch_x = frame.content_area.x2 - switch_width;
        let switch_y = cursor_y + (fonts.line_height_body() - switch_height) / 2;

        let mut track = RectDsc::new();
        track.bg_color = if toggle.enabled {
            palette.accent_yellow
        } else {
            palette.accent_light
        };
        track.bg_opa = OPA_COVER;
        track.radius = switch_height / 2;
        let track_area = Area::new(
            switch_x,
            switch_y,
            switch_x + switch_width,
            switch_y + switch_height,
        );
        draw_rect(surface, &track, &track_area);

        let knob_diameter = switch_height;
        let knob_x = if toggle.enabled {
            switch_x + switch_width - knob_diameter
        } else {
            switch_x
        };
        let knob_area = Area::new(
            knob_x,
            switch_y,
            knob_x + knob_diameter,
            switch_y + knob_diameter,
        );
        let mut knob = RectDsc::new();
        knob.bg_color = palette.container;
        knob.bg_opa = OPA_COVER;
        knob.radius = RADIUS_CIRCLE;
        draw_rect(surface, &knob, &knob_area);

        cursor_y += fonts.line_height_body() + metrics.section_padding / 2;
    }
}

pub fn draw_button(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    top: i32,
    config: ButtonConfig,
) -> Area {
    let width = metrics.content_width();
    let left = metrics.content_x();
    let height = metrics.button_height;
    let area = Area::new(left, top, left + width - 1, top + height - 1);

    let mut button = RectDsc::new();
    match config.kind {
        ButtonKind::Primary => {
            button.bg_color = palette.accent_dark;
            button.bg_grad =
                Gradient::vertical(palette.accent_dark, Rgba8888::rgba(20, 20, 20, 255));
            button.bg_opa = OPA_COVER;
            button.radius = metrics.section_radius;
            draw_rect(surface, &button, &area);
            let text_y = area.y1 + (height - fonts.line_height_body()) / 2;
            let text_width = measure_text_with_font(config.text, 0, fonts.body);
            let text_x = area.x1 + (width - text_width) / 2;
            draw_text(
                surface,
                config.text,
                fonts.body,
                text_x,
                text_y,
                palette.container,
            );
        }
        ButtonKind::Secondary => {
            button.bg_color = palette.container;
            button.bg_opa = OPA_COVER;
            button.radius = metrics.section_radius;
            button.border_width = 1;
            button.border_color = palette.accent_dark;
            button.border_opa = OPA_COVER;
            draw_rect(surface, &button, &area);
            let text_y = area.y1 + (height - fonts.line_height_body()) / 2;
            let text_width = measure_text_with_font(config.text, 0, fonts.body);
            let text_x = area.x1 + (width - text_width) / 2;
            draw_text(
                surface,
                config.text,
                fonts.body,
                text_x,
                text_y,
                palette.text_primary,
            );
        }
        ButtonKind::Icon | ButtonKind::IconSecondary => {
            let diameter = height.min(width);
            let size_area = Area::new(
                area.x1 + (width - diameter) / 2,
                area.y1,
                area.x1 + (width + diameter) / 2,
                area.y1 + diameter,
            );
            button.bg_color = if matches!(config.kind, ButtonKind::Icon) {
                palette.accent_dark
            } else {
                palette.container
            };
            button.bg_opa = OPA_COVER;
            button.radius = RADIUS_CIRCLE;
            button.border_width = if matches!(config.kind, ButtonKind::IconSecondary) {
                1
            } else {
                0
            };
            button.border_color = palette.outline;
            button.border_opa = if button.border_width > 0 {
                OPA_COVER
            } else {
                0
            };
            draw_rect(surface, &button, &size_area);
            let text_width = measure_text_with_font(config.text, 0, fonts.title);
            let text_x = size_area.x1 + (diameter - text_width) / 2;
            let text_y = size_area.y1 + (diameter - fonts.line_height_title()) / 2;
            let text_color = if matches!(config.kind, ButtonKind::Icon) {
                palette.container
            } else {
                palette.text_primary
            };
            draw_text(
                surface,
                config.text,
                fonts.title,
                text_x,
                text_y,
                text_color,
            );
            return size_area;
        }
    }

    area
}

pub fn draw_list(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    fonts: StyleFonts,
    palette: StylePalette,
    lines: &[&str],
) {
    let mut y = frame.content_area.y1;
    let width = frame.content_width();
    for (idx, line) in lines.iter().enumerate() {
        let text_y = y;
        let _ = draw_text(
            surface,
            line,
            fonts.body,
            frame.content_area.x1,
            text_y,
            palette.text_primary,
        );
        y += fonts.line_height_body() + 2;
        if idx + 1 < lines.len() {
            let divider = Area::new(
                frame.content_area.x1,
                y,
                frame.content_area.x1 + width - 1,
                y,
            );
            let mut dsc = RectDsc::new();
            dsc.bg_color = palette.outline;
            dsc.bg_opa = 80;
            draw_rect(surface, &dsc, &divider);
            y += 2;
        }
    }
}

pub fn draw_badge(
    surface: &mut DrawingSurface,
    fonts: StyleFonts,
    palette: StylePalette,
    badge: BadgeConfig,
    right_edge: i32,
    baseline: i32,
) -> i32 {
    let text = badge.text;
    let font = fonts.small;
    let text_width = measure_text_with_font(text, 0, font);
    if text_width <= 0 {
        return 0;
    }
    let padding_x = 4;
    let padding_y = 2;
    let height = fonts.line_height_small() + padding_y * 2;
    let width = text_width + padding_x * 2;
    let x1 = right_edge - width;
    let y1 = baseline - height;

    let (bg, fg) = match badge.tone {
        BadgeTone::Accent => (palette.accent_yellow, palette.accent_dark),
        BadgeTone::Gray => (palette.accent_gray, palette.container),
        BadgeTone::Danger => (Rgba8888::rgba(255, 59, 48, 255), palette.container),
    };

    let mut pill = RectDsc::new();
    pill.bg_color = bg;
    pill.bg_opa = OPA_COVER;
    pill.radius = height / 2;
    let area = Area::new(x1, y1, x1 + width, y1 + height);
    draw_rect(surface, &pill, &area);

    let text_x = x1 + padding_x;
    let text_y = y1 + padding_y;
    draw_text(surface, text, font, text_x, text_y, fg);
    width
}

fn draw_battery(
    surface: &mut DrawingSurface,
    bar_area: &Area,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    percent: u8,
) {
    let height = fonts.line_height_status() - 2;
    let width = height + 6;
    let x2 = bar_area.x2 - metrics.section_padding;
    let x1 = x2 - width;
    let y1 = bar_area.y1 + metrics.section_padding / 2;
    let y2 = y1 + height;

    let mut outline = RectDsc::new();
    outline.bg_color = palette.container;
    outline.bg_opa = 0;
    outline.border_width = 1;
    outline.border_color = palette.container;
    outline.border_opa = OPA_COVER;
    outline.radius = 2;
    let area = Area::new(x1, y1, x2, y2);
    draw_rect(surface, &outline, &area);

    let knob_width = width / 6;
    let knob_area = Area::new(
        x2 + 1,
        y1 + height / 3,
        x2 + 1 + knob_width,
        y2 - height / 3,
    );
    let mut knob = RectDsc::new();
    knob.bg_color = palette.container;
    knob.bg_opa = OPA_COVER;
    knob.radius = 1;
    draw_rect(surface, &knob, &knob_area);

    let fill_percent = percent.min(100);
    if fill_percent > 0 {
        let inner_width = width - 3;
        let fill_width = (inner_width as u32 * fill_percent as u32 / 100) as i32;
        let fill_area = Area::new(x1 + 2, y1 + 2, x1 + 1 + fill_width, y2 - 2);
        let mut fill = RectDsc::new();
        fill.bg_color = palette.accent_yellow;
        fill.bg_opa = OPA_COVER;
        fill.radius = 1;
        draw_rect(surface, &fill, &fill_area);
    }
}

pub fn draw_text(
    surface: &mut DrawingSurface,
    text: &str,
    font: FontId,
    x: i32,
    y: i32,
    color: Rgba8888,
) -> i32 {
    use rust_gfx::primitives::label::line_height_for_font;

    if text.is_empty() {
        return 0;
    }

    let width = measure_text_with_font(text, 0, font);
    if width <= 0 {
        return 0;
    }

    let height = line_height_for_font(font);
    let mut label = LabelDsc::new(String::from(text));
    label.font = font;
    label.color = color;
    let area = Area::new(x, y, x + width - 1, y + height - 1);
    draw_label(surface, &label, &area);
    width
}
