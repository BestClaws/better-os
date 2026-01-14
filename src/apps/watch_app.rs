//! Minimalist watch face optimized for rectangular displays.

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{
    RectDsc,
    arc::{ArcDsc, draw_arc},
    draw_label, draw_line, draw_rect,
    label::{LabelDsc, line_height, measure_text},
    line::LineDsc,
};
use rust_gfx::rasterizer::Rasterizer;
use rust_gfx::types::{Area, Gradient, OPA_COVER, Point, RADIUS_CIRCLE};

extern crate alloc;
use alloc::format;

#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting watch app");
    let start = Instant::now();
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let width = surface.width() as i32;
            let height = surface.height() as i32;
            let cx = width / 2;
            let cy = height / 2;

            let elapsed = Instant::now() - start;
            let total_micros = elapsed.as_micros();
            let second_fraction = (total_micros as f32 / 1_000_000.0) % 60.0;
            let total_seconds = (total_micros / 1_000_000) as i32;
            let minutes = (total_seconds / 60) % 60;
            let hours = (total_seconds / 3600) % 24;
            let minute_fraction = minutes as f32 + second_fraction / 60.0;
            let hour_fraction = hours as f32 + minute_fraction / 60.0;

            draw_background(surface, width, height);
            if let Some(layout) = compute_watch_layout(width, height) {
                draw_watch_face(
                    surface,
                    &layout,
                    hour_fraction,
                    minute_fraction,
                    second_fraction,
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
    // Deep gradient background
    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(8, 8, 12, 255);
    bg.bg_opa = OPA_COVER;
    bg.bg_grad = Gradient::vertical(
        Rgba8888::rgba(12, 12, 18, 255),
        Rgba8888::rgba(4, 4, 8, 255),
    );
    let bg_area = Area::new(0, 0, width, height);
    draw_rect(surface, &bg, &bg_area);
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

    Some(WatchLayout { cx, cy, radius, text_y })
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

    let mut ring = ArcDsc::new(Point::new(layout.cx, layout.cy), layout.radius, 0, 360);
    ring.width = 2;
    ring.color = Rgba8888::rgba(220, 225, 240, 255);
    ring.opa = OPA_COVER;
    ring.rounded = true;
    draw_arc(surface, &ring);

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

fn draw_center_hub(surface: &mut DrawingSurface, cx: i32, cy: i32) {
    let mut hub = RectDsc::new();
    hub.bg_color = Rgba8888::rgba(235, 240, 250, 255);
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
    let x1 = layout.cx - text_width / 2;
    let y1 = layout.text_y;
    let area = Area::new(x1, y1, x1 + text_width - 1, y1 + text_height - 1);
    let mut label = LabelDsc::new(text);
    label.color = Rgba8888::rgba(230, 235, 245, 255);
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
    hour_hand.color = Rgba8888::rgba(200, 210, 255, 255);
    draw_line(surface, &hour_hand);

    let mut minute_hand = LineDsc::new(Point::new(cx, cy), minute_end);
    minute_hand.width = (radius / 16).max(2);
    minute_hand.round_start = true;
    minute_hand.round_end = true;
    minute_hand.color = Rgba8888::rgba(150, 240, 200, 230);
    draw_line(surface, &minute_hand);

    let mut second_hand = LineDsc::new(Point::new(cx, cy), second_end);
    second_hand.width = 1;
    second_hand.round_end = true;
    second_hand.color = Rgba8888::rgba(255, 150, 170, 220);
    draw_line(surface, &second_hand);
}

fn angle_point(cx: i32, cy: i32, radius: i32, angle_deg: f32) -> Point {
    let angle_rad = angle_deg.to_radians();
    let x = cx + (radius as f32 * angle_rad.cos()) as i32;
    let y = cy + (radius as f32 * angle_rad.sin()) as i32;
    Point::new(x, y)
}
