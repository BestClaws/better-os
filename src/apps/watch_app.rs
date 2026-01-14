//! Minimalist watch face optimized for rectangular displays.

use crate::system::app::app_context::AppContext;
use crate::system::services::rtc_srv::current_datetime;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{
    arc::{draw_arc, ArcDsc},
    draw_label, draw_line, draw_rect,
    label::{line_height, measure_text, LabelDsc},
    line::LineDsc,
    RectDsc,
};
use rust_gfx::types::{Area, Gradient, Point, OPA_COVER, RADIUS_CIRCLE};

extern crate alloc;
use alloc::format;

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
        let fallback_micros = fallback_elapsed.as_micros();

        let (hours, minutes, hour_progress, minute_progress, second_progress) = match rtc_snapshot {
            Some(dt) => {
                let second_progress = dt.second as f32;
                let minute_progress = dt.minute as f32 + second_progress / 60.0;
                let hour_progress = dt.hour as f32 + minute_progress / 60.0;
                (
                    dt.hour as i32,
                    dt.minute as i32,
                    hour_progress,
                    minute_progress,
                    second_progress,
                )
            }
            None => {
                let second_progress = (fallback_micros as f32 / 1_000_000.0) % 60.0;
                let total_seconds = (fallback_micros / 1_000_000) as i32;
                let minutes = (total_seconds / 60) % 60;
                let hours = (total_seconds / 3600) % 24;
                let minute_progress = minutes as f32 + second_progress / 60.0;
                let hour_progress = hours as f32 + minute_progress / 60.0;
                (
                    hours,
                    minutes,
                    hour_progress,
                    minute_progress,
                    second_progress,
                )
            }
        };

        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let width = surface.width() as i32;
            let height = surface.height() as i32;
            let cx = width / 2;
            let cy = height / 2;

            draw_background(surface, width, height);
            if let Some(layout) = compute_watch_layout(width, height) {
                draw_watch_face(
                    surface,
                    &layout,
                    hour_progress,
                    minute_progress,
                    second_progress,
                );
                draw_bottom_label(surface, &layout, hours, minutes);
            }
        })
        .await;

        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

fn draw_background(surface: &mut DrawingSurface, width: i32, height: i32) {
    if width <= 0 || height <= 0 {
        return;
    }

    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(248, 249, 252, 255);
    bg.bg_opa = OPA_COVER;
    bg.bg_grad = Gradient::vertical(
        Rgba8888::rgba(244, 246, 252, 255),
        Rgba8888::rgba(226, 228, 236, 255),
    );
    let bg_area = Area::new(0, 0, width - 1, height - 1);
    draw_rect(surface, &bg, &bg_area);

    draw_light_grid(surface, width, height);
    draw_header_accents(surface, width);
}

fn draw_light_grid(surface: &mut DrawingSurface, width: i32, height: i32) {
    let spacing = (width.min(height) / 10).max(18);
    let mut line = RectDsc::new();
    line.bg_color = Rgba8888::rgba(200, 202, 210, 40);
    line.bg_opa = OPA_COVER;

    for y in (spacing..height).step_by(spacing as usize) {
        let area = Area::new(0, y, width - 1, y);
        draw_rect(surface, &line, &area);
    }

    for x in (spacing..width).step_by(spacing as usize) {
        let area = Area::new(x, 0, x, height - 1);
        draw_rect(surface, &line, &area);
    }
}

fn draw_header_accents(surface: &mut DrawingSurface, width: i32) {
    let accent_width = (width / 30).max(4).min(12);
    let accent_height = accent_width / 2;
    let colors = [
        Rgba8888::rgba(236, 70, 170, 255),
        Rgba8888::rgba(70, 190, 235, 255),
        Rgba8888::rgba(254, 211, 64, 255),
    ];
    let spacing = accent_width + 2;
    let total_width = spacing * colors.len() as i32 - 2;
    let start_x = ((width - total_width) / 2).max(0);
    let y = 4;

    for (idx, color) in colors.into_iter().enumerate() {
        let mut bar = RectDsc::new();
        bar.bg_color = color;
        bar.bg_opa = OPA_COVER;
        bar.radius = 1;
        let x1 = start_x + spacing * idx as i32;
        let area = Area::new(x1, y, x1 + accent_width - 1, y + accent_height - 1);
        draw_rect(surface, &bar, &area);
    }
}

struct WatchLayout {
    cx: i32,
    cy: i32,
    radius: i32,
    text_y: i32,
}

fn compute_watch_layout(width: i32, height: i32) -> Option<WatchLayout> {
    let text_height = line_height();
    let top_margin = 8;
    let bottom_margin = 6;
    if height <= text_height + top_margin + bottom_margin {
        return None;
    }

    let available_height = height - text_height - bottom_margin - top_margin;
    if available_height <= 0 {
        return None;
    }

    let diameter_cap = available_height.min(125);
    let diameter = width.min(102).min(diameter_cap);
    if diameter < 4 {
        return None;
    }

    let radius = diameter / 2;
    let cx = width / 2;
    let cy = top_margin + radius;
    let text_y = height - bottom_margin - text_height;

    Some(WatchLayout {
        cx,
        cy,
        radius,
        text_y,
    })
}

fn draw_watch_face(
    surface: &mut DrawingSurface,
    layout: &WatchLayout,
    hour_progress: f32,
    minute_progress: f32,
    second_progress: f32,
) {
    if layout.radius <= 0 {
        return;
    }

    draw_face_plate(surface, layout.cx, layout.cy, layout.radius);

    let mut ring = ArcDsc::new(Point::new(layout.cx, layout.cy), layout.radius, 0, 360);
    ring.width = 3;
    ring.color = Rgba8888::rgba(92, 96, 112, 220);
    ring.opa = OPA_COVER;
    ring.rounded = true;
    draw_arc(surface, &ring);

    draw_accent_arcs(surface, layout.cx, layout.cy, layout.radius);

    draw_hands(
        surface,
        layout.cx,
        layout.cy,
        layout.radius,
        hour_progress,
        minute_progress,
        second_progress,
    );

    draw_center_hub(surface, layout.cx, layout.cy);
}

fn draw_face_plate(surface: &mut DrawingSurface, cx: i32, cy: i32, radius: i32) {
    if radius <= 2 {
        return;
    }

    let inset = radius - 4;
    if inset <= 0 {
        return;
    }

    let mut plate = RectDsc::new();
    plate.bg_color = Rgba8888::rgba(252, 253, 255, 255);
    plate.bg_opa = OPA_COVER;
    plate.radius = RADIUS_CIRCLE;
    let area = Area::new(cx - inset, cy - inset, cx + inset, cy + inset);
    draw_rect(surface, &plate, &area);
}

fn draw_accent_arcs(surface: &mut DrawingSurface, cx: i32, cy: i32, radius: i32) {
    if radius <= 6 {
        return;
    }

    let arc_radius = radius - 2;
    let accents = [
        (18, 40, Rgba8888::rgba(236, 70, 170, 255)),
        (122, 38, Rgba8888::rgba(70, 190, 235, 255)),
        (242, 46, Rgba8888::rgba(254, 211, 64, 255)),
    ];

    for (start, sweep, color) in accents {
        // stylized color pops
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
        inner_ring.color = Rgba8888::rgba(172, 176, 188, 200);
        inner_ring.opa = OPA_COVER;
        inner_ring.rounded = true;
        draw_arc(surface, &inner_ring);
    }
}

fn draw_center_hub(surface: &mut DrawingSurface, cx: i32, cy: i32) {
    let mut hub = RectDsc::new();
    hub.bg_color = Rgba8888::rgba(64, 68, 84, 255);
    hub.bg_opa = OPA_COVER;
    hub.radius = RADIUS_CIRCLE;
    let hub_area = Area::new(cx - 2, cy - 2, cx + 2, cy + 2);
    draw_rect(surface, &hub, &hub_area);
}

fn draw_bottom_label(surface: &mut DrawingSurface, layout: &WatchLayout, hours: i32, minutes: i32) {
    let text = format!("{:02}:{:02}", hours, minutes);
    let text_width = measure_text(&text, 0);
    if text_width <= 0 {
        return;
    }

    let text_height = line_height();
    let pill_padding_x = 8;
    let pill_padding_y = 2;
    let pill_width = text_width + pill_padding_x * 2;
    let pill_height = text_height + pill_padding_y * 2;
    let pill_x1 = layout.cx - pill_width / 2;
    let pill_y1 = layout.text_y - pill_padding_y;
    let pill_area = Area::new(
        pill_x1,
        pill_y1,
        pill_x1 + pill_width - 1,
        pill_y1 + pill_height - 1,
    );

    let mut pill = RectDsc::new();
    pill.bg_color = Rgba8888::rgba(248, 249, 252, 200);
    pill.bg_opa = OPA_COVER;
    pill.radius = pill_height / 2;
    draw_rect(surface, &pill, &pill_area);

    let x1 = layout.cx - text_width / 2;
    let y1 = pill_y1 + pill_padding_y;
    let area = Area::new(x1, y1, x1 + text_width - 1, y1 + text_height - 1);
    let mut label = LabelDsc::new(text);
    label.color = Rgba8888::rgba(60, 64, 80, 255);
    draw_label(surface, &label, &area);
}

fn draw_hands(
    surface: &mut DrawingSurface,
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
    hour_hand.color = Rgba8888::rgba(86, 92, 110, 255);
    draw_line(surface, &hour_hand);

    let mut minute_hand = LineDsc::new(Point::new(cx, cy), minute_end);
    minute_hand.width = (radius / 16).max(2);
    minute_hand.round_start = true;
    minute_hand.round_end = true;
    minute_hand.color = Rgba8888::rgba(40, 44, 60, 255);
    draw_line(surface, &minute_hand);

    let mut second_hand = LineDsc::new(Point::new(cx, cy), second_end);
    second_hand.width = 2;
    second_hand.round_end = true;
    second_hand.color = Rgba8888::rgba(254, 211, 64, 255);
    draw_line(surface, &second_hand);
}

fn angle_point(cx: i32, cy: i32, radius: i32, angle_deg: f32) -> Point {
    let angle_rad = angle_deg.to_radians();
    let x = cx + (radius as f32 * angle_rad.cos()) as i32;
    let y = cy + (radius as f32 * angle_rad.sin()) as i32;
    Point::new(x, y)
}
