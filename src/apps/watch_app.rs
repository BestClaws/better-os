//! Arknights-inspired watch face with reusable UI components.

use crate::apps::components::{
    draw_background, draw_card, draw_progress_bar, draw_quick_actions, draw_status_bar, draw_text,
    draw_time_display, draw_toggle, AccentColor, BadgeConfig, BadgeTone, CardConfig, CardFrame,
    ProgressBarConfig, QuickAction, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
    TimeDisplayData, ToggleConfig,
};
use crate::system::app::app_context::AppContext;
use crate::system::hal::rtc::RtcDateTime;
use crate::system::services::rtc_srv::current_datetime;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::label::measure_text_with_font;
use rust_gfx::primitives::{
    arc::{draw_arc, ArcDsc},
    line::{draw_line, LineDsc},
    rectangle::{draw_rect, RectDsc},
};
use rust_gfx::types::{Area, Gradient, Point, OPA_COVER, RADIUS_CIRCLE};

extern crate alloc;
use alloc::{format, string::String};

#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting watch app");
    let fallback_start = Instant::now();
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let rtc_snapshot = current_datetime().await;
        let fallback_elapsed = Instant::now() - fallback_start;
        let fallback_micros = fallback_elapsed.as_micros() as u128;
        let time_state = TimeState::from_snapshot(rtc_snapshot, fallback_micros);

        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            draw_watch_view(surface, &time_state);
        })
        .await;

        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

const WEEKDAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const FACE_INSET: i32 = 4;

#[derive(Clone, Debug)]
struct TimeState {
    hours: i32,
    minutes: i32,
    hour_progress: f32,
    minute_progress: f32,
    second_progress: f32,
    status_text: String,
    time_text: String,
    date_text: String,
    minute_percent: u8,
}

impl TimeState {
    fn from_snapshot(snapshot: Option<RtcDateTime>, fallback_micros: u128) -> Self {
        match snapshot {
            Some(dt) => {
                let second_progress = dt.second as f32;
                let minute_progress = dt.minute as f32 + second_progress / 60.0;
                let hour_progress = dt.hour as f32 + minute_progress / 60.0;
                let status_text = format!("{:02}:{:02}", dt.hour, dt.minute);
                let time_text = status_text.clone();

                let weekday = WEEKDAY_NAMES[dt.weekday as usize % WEEKDAY_NAMES.len()];
                let month_idx = dt.month.saturating_sub(1).min(11);
                let month = MONTH_NAMES[month_idx as usize];
                let date_text = format!("{}, {} {:02}", weekday, month, dt.day);

                let minute_percent = ((minute_progress % 60.0) / 60.0 * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8;

                Self {
                    hours: dt.hour as i32,
                    minutes: dt.minute as i32,
                    hour_progress,
                    minute_progress,
                    second_progress,
                    status_text,
                    time_text,
                    date_text,
                    minute_percent,
                }
            }
            None => {
                let seconds = (fallback_micros as f32 / 1_000_000.0) % 60.0;
                let total_seconds = (fallback_micros / 1_000_000) as i32;
                let minutes = (total_seconds / 60) % 60;
                let hours = (total_seconds / 3600) % 24;
                let minute_progress = minutes as f32 + seconds / 60.0;
                let hour_progress = hours as f32 + minute_progress / 60.0;
                let status_text = format!("{:02}:{:02}", hours, minutes);
                let minute_percent = ((minute_progress % 60.0) / 60.0 * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8;

                Self {
                    hours,
                    minutes,
                    hour_progress,
                    minute_progress,
                    second_progress: seconds,
                    status_text: status_text.clone(),
                    time_text: status_text,
                    date_text: String::from("Syncing..."),
                    minute_percent,
                }
            }
        }
    }
}

fn draw_watch_view(surface: &mut DrawingSurface, state: &TimeState) {
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: state.status_text.as_str(),
            battery_percent: 72,
            right: Some("LIVE"),
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;

    let time_area = draw_time_display(
        surface,
        &metrics,
        fonts,
        palette,
        TimeDisplayData {
            time_text: state.time_text.as_str(),
            date_text: state.date_text.as_str(),
        },
        next_y,
    );

    next_y = time_area.y2 + 1 + metrics.section_spacing;

    let quick_actions = [
        QuickAction {
            icon: "▶",
            label: "Play",
        },
        QuickAction {
            icon: "■",
            label: "Stats",
        },
        QuickAction {
            icon: "●",
            label: "Team",
        },
        QuickAction {
            icon: "★",
            label: "Gift",
        },
    ];

    let toggles = [
        ToggleConfig {
            label: "Notify",
            enabled: true,
        },
        ToggleConfig {
            label: "Haptic",
            enabled: true,
        },
        ToggleConfig {
            label: "Sound",
            enabled: state.hours >= 6 && state.hours <= 22,
        },
    ];

    let action_rows = ((quick_actions.len() as i32 + 1) / 2).max(1);
    let actions_height =
        action_rows * metrics.button_height + (action_rows - 1).max(0) * metrics.quick_gap;
    let toggles_height =
        toggles.len() as i32 * (fonts.line_height_body() + metrics.section_padding / 2);
    let progress_height = fonts.line_height_small() + metrics.section_padding * 3;
    let base_quick_height =
        (actions_height + toggles_height + progress_height + metrics.section_padding * 2)
            .max(metrics.button_height * 2);

    let available = metrics.height - next_y - metrics.section_spacing;
    if available <= metrics.button_height {
        let watch_card = draw_card(
            surface,
            &metrics,
            fonts,
            palette,
            next_y,
            CardConfig {
                title: Some("Dial"),
                subtitle: Some("Analog"),
                badge: Some(BadgeConfig {
                    text: "LIVE",
                    tone: BadgeTone::Accent,
                }),
                accent: AccentColor::Yellow,
                height: available.max(metrics.button_height),
            },
        );
        draw_watch_dial(surface, &watch_card, &metrics, fonts, palette, state);
        return;
    }

    if available <= metrics.button_height * 2 {
        let watch_card = draw_card(
            surface,
            &metrics,
            fonts,
            palette,
            next_y,
            CardConfig {
                title: Some("Dial"),
                subtitle: Some("Analog"),
                badge: Some(BadgeConfig {
                    text: "LIVE",
                    tone: BadgeTone::Accent,
                }),
                accent: AccentColor::Yellow,
                height: available,
            },
        );
        draw_watch_dial(surface, &watch_card, &metrics, fonts, palette, state);
        return;
    }

    let mut watch_height = (available * 2) / 3;
    if watch_height < metrics.button_height {
        watch_height = metrics.button_height;
    }
    if available - watch_height < metrics.button_height {
        watch_height = available - metrics.button_height;
    }
    if watch_height < metrics.button_height {
        watch_height = metrics.button_height;
    }

    let mut quick_card_height = available - watch_height;
    if quick_card_height < metrics.button_height {
        quick_card_height = metrics.button_height;
        watch_height = available - quick_card_height;
    }
    if quick_card_height > base_quick_height {
        quick_card_height = base_quick_height.min(available);
        watch_height = available - quick_card_height;
    }
    if watch_height <= 0 {
        watch_height = available;
        quick_card_height = 0;
    }

    let watch_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Dial"),
            subtitle: Some("Analog"),
            badge: Some(BadgeConfig {
                text: "LIVE",
                tone: BadgeTone::Accent,
            }),
            accent: AccentColor::Yellow,
            height: watch_height,
        },
    );

    draw_watch_dial(surface, &watch_card, &metrics, fonts, palette, state);

    next_y = watch_card.next_y(&metrics);

    if next_y >= metrics.height {
        return;
    }

    if quick_card_height < metrics.button_height {
        return;
    }

    let quick_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Control"),
            subtitle: Some("Quick access"),
            badge: None,
            accent: AccentColor::Gray,
            height: quick_card_height,
        },
    );

    let mut actions_area = quick_card.content_area;
    let available_height = actions_area.y2 - actions_area.y1;
    let adjusted_actions_height = actions_height.min(available_height.max(0));
    actions_area.y2 = actions_area.y1 + adjusted_actions_height;
    let actions_frame = CardFrame {
        card_area: quick_card.card_area,
        content_area: actions_area,
    };
    draw_quick_actions(
        surface,
        &actions_frame,
        &metrics,
        fonts,
        palette,
        &quick_actions,
    );

    let mut progress_area = quick_card.content_area;
    progress_area.y1 = actions_area.y2 + metrics.section_padding;
    progress_area.y2 = (progress_area.y1 + progress_height).min(quick_card.content_area.y2);
    if progress_area.y1 < progress_area.y2 {
        let progress_frame = CardFrame {
            card_area: quick_card.card_area,
            content_area: progress_area,
        };
        draw_progress_bar(
            surface,
            &progress_frame,
            &metrics,
            fonts,
            palette,
            ProgressBarConfig {
                percent: state.minute_percent,
                label: "Hour",
            },
        );
    }

    let mut toggle_area = quick_card.content_area;
    toggle_area.y1 = progress_area.y2 + metrics.section_padding;
    if toggle_area.y1 < toggle_area.y2 {
        let toggles_frame = CardFrame {
            card_area: quick_card.card_area,
            content_area: toggle_area,
        };
        draw_toggle(surface, &toggles_frame, &metrics, fonts, palette, &toggles);
    }
}

fn draw_watch_dial(
    surface: &mut DrawingSurface,
    frame: &CardFrame,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    state: &TimeState,
) {
    let area = frame.content_area;
    let width = area.x2 - area.x1 + 1;
    let height = area.y2 - area.y1 + 1;
    if width <= 0 || height <= 0 {
        return;
    }

    let label_height = fonts.line_height_body() + metrics.section_padding * 2;
    let face_height = (height - label_height)
        .max(metrics.section_padding * 4)
        .min(height);
    if face_height <= 0 {
        return;
    }
    let radius = (width.min(face_height) / 2).saturating_sub(metrics.section_padding);
    if radius <= 4 {
        return;
    }

    let cx = area.x1 + width / 2;
    let cy = area.y1 + face_height / 2;

    draw_face_plate(surface, palette, cx, cy, radius);

    let mut ring = ArcDsc::new(Point::new(cx, cy), radius, 0, 360);
    ring.width = 3;
    ring.color = palette.accent_gray;
    ring.opa = OPA_COVER;
    ring.rounded = true;
    draw_arc(surface, &ring);

    draw_accent_arcs(surface, palette, cx, cy, radius);
    draw_hands(
        surface,
        palette,
        cx,
        cy,
        radius,
        state.hour_progress,
        state.minute_progress,
        state.second_progress,
    );
    draw_center_hub(surface, palette, cx, cy);

    let pill_area = Area::new(area.x1, area.y2 - label_height, area.x2, area.y2);
    draw_time_pill(
        surface,
        fonts,
        palette,
        metrics,
        &pill_area,
        state.time_text.as_str(),
    );
}

fn draw_time_pill(
    surface: &mut DrawingSurface,
    fonts: StyleFonts,
    palette: StylePalette,
    metrics: &StyleMetrics,
    area: &Area,
    text: &str,
) {
    let text_width = measure_text_with_font(text, 0, fonts.body);
    if text_width <= 0 {
        return;
    }

    let pill_padding_x = metrics.section_padding * 2;
    let pill_padding_y = metrics.section_padding;
    let pill_width = text_width + pill_padding_x * 2;
    let pill_height = fonts.line_height_body() + pill_padding_y * 2;
    let center_x = (area.x1 + area.x2) / 2;
    let x1 = center_x - pill_width / 2;
    let y1 = area.y1 + (area.y2 - area.y1 - pill_height) / 2;
    let pill_area = Area::new(x1, y1, x1 + pill_width - 1, y1 + pill_height - 1);

    let mut pill = RectDsc::new();
    pill.bg_color = palette.container_alt;
    pill.bg_opa = OPA_COVER;
    pill.radius = pill_height / 2;
    pill.border_width = 1;
    pill.border_color = palette.outline;
    pill.border_opa = OPA_COVER;
    draw_rect(surface, &pill, &pill_area);

    let text_x = x1 + pill_padding_x;
    let text_y = y1 + pill_padding_y;
    draw_text(
        surface,
        text,
        fonts.body,
        text_x,
        text_y,
        palette.text_primary,
    );
}

fn draw_face_plate(
    surface: &mut DrawingSurface,
    palette: StylePalette,
    cx: i32,
    cy: i32,
    radius: i32,
) {
    let inset = radius.saturating_sub(FACE_INSET);
    if inset <= 0 {
        return;
    }

    let mut plate = RectDsc::new();
    plate.bg_color = palette.container;
    plate.bg_grad = Gradient::vertical(palette.container, palette.container_alt);
    plate.bg_opa = OPA_COVER;
    plate.radius = RADIUS_CIRCLE;
    let area = Area::new(cx - inset, cy - inset, cx + inset, cy + inset);
    draw_rect(surface, &plate, &area);
}

fn draw_accent_arcs(
    surface: &mut DrawingSurface,
    palette: StylePalette,
    cx: i32,
    cy: i32,
    radius: i32,
) {
    if radius <= 6 {
        return;
    }

    let arc_radius = radius - 2;
    let accents = [
        (18, 40, palette.accent_yellow),
        (138, 32, Rgba8888::rgba(80, 120, 180, 255)),
        (252, 30, Rgba8888::rgba(90, 200, 240, 255)),
    ];

    for (start, sweep, color) in accents {
        let mut accent = ArcDsc::new(Point::new(cx, cy), arc_radius, start, start + sweep);
        accent.width = 4;
        accent.color = color;
        accent.opa = OPA_COVER;
        accent.rounded = true;
        draw_arc(surface, &accent);
    }

    if arc_radius > 6 {
        let mut inner_ring = ArcDsc::new(Point::new(cx, cy), arc_radius - 6, 0, 360);
        inner_ring.width = 1;
        inner_ring.color = palette.accent_light;
        inner_ring.opa = OPA_COVER;
        inner_ring.rounded = true;
        draw_arc(surface, &inner_ring);
    }
}

fn draw_center_hub(surface: &mut DrawingSurface, palette: StylePalette, cx: i32, cy: i32) {
    let mut hub = RectDsc::new();
    hub.bg_color = palette.accent_dark;
    hub.bg_opa = OPA_COVER;
    hub.radius = RADIUS_CIRCLE;
    let hub_area = Area::new(cx - 2, cy - 2, cx + 2, cy + 2);
    draw_rect(surface, &hub, &hub_area);
}

fn draw_hands(
    surface: &mut DrawingSurface,
    palette: StylePalette,
    cx: i32,
    cy: i32,
    radius: i32,
    hour_progress: f32,
    minute_progress: f32,
    second_progress: f32,
) {
    if radius <= 6 {
        return;
    }

    let hour_fraction = (hour_progress % 12.0) / 12.0;
    let minute_fraction = (minute_progress % 60.0) / 60.0;
    let second_fraction = (second_progress % 60.0) / 60.0;

    let hour_angle = hour_fraction * 360.0 - 90.0;
    let minute_angle = minute_fraction * 360.0 - 90.0;
    let second_angle = second_fraction * 360.0 - 90.0;

    let hour_length = (radius as f32 * 0.55) as i32;
    let minute_length = (radius as f32 * 0.82) as i32;
    let second_length = (radius as f32 * 0.9) as i32;

    let hour_end = angle_point(cx, cy, hour_length, hour_angle);
    let minute_end = angle_point(cx, cy, minute_length, minute_angle);
    let second_end = angle_point(cx, cy, second_length, second_angle);

    let mut hour_hand = LineDsc::new(Point::new(cx, cy), hour_end);
    hour_hand.width = (radius / 12).max(3);
    hour_hand.round_start = true;
    hour_hand.round_end = true;
    hour_hand.color = palette.text_secondary;
    draw_line(surface, &hour_hand);

    let mut minute_hand = LineDsc::new(Point::new(cx, cy), minute_end);
    minute_hand.width = (radius / 16).max(2);
    minute_hand.round_start = true;
    minute_hand.round_end = true;
    minute_hand.color = palette.text_primary;
    draw_line(surface, &minute_hand);

    let mut second_hand = LineDsc::new(Point::new(cx, cy), second_end);
    second_hand.width = 2;
    second_hand.round_end = true;
    second_hand.color = palette.accent_yellow;
    draw_line(surface, &second_hand);
}

fn angle_point(cx: i32, cy: i32, radius: i32, angle_deg: f32) -> Point {
    let angle_rad = angle_deg.to_radians();
    let x = cx + (radius as f32 * angle_rad.cos()) as i32;
    let y = cy + (radius as f32 * angle_rad.sin()) as i32;
    Point::new(x, y)
}
