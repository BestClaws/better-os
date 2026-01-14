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
            let seconds = total_seconds % 60;
            let minutes = (total_seconds / 60) % 60;
            let hours = (total_seconds / 3600) % 24;
            let minute_fraction = minutes as f32 + second_fraction / 60.0;
            let hour_fraction = hours as f32 + minute_fraction / 60.0;

            draw_background(surface, width, height);
            let inner_radius = draw_time_arcs(
                surface,
                width,
                height,
                hour_fraction,
                minute_fraction,
                second_fraction,
            );
            draw_center_info(
                surface,
                cx,
                cy,
                inner_radius,
                hours,
                minutes,
                seconds,
                hour_fraction,
                minute_fraction,
                second_fraction,
                total_seconds / 60,
            );
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

fn draw_time_arcs(
    surface: &mut DrawingSurface,
    width: i32,
    height: i32,
    hours: f32,
    minutes: f32,
    seconds: f32,
) -> i32 {
    let cx = width / 2;
    let cy = height / 2;

    // Calculate base radius from smaller dimension
    let mut base_radius = (width.min(height) as f32 * 0.42) as i32;
    base_radius = base_radius.max(28);

    // Arc parameters - concentric rings
    let mut arc_width = (base_radius as f32 * 0.18) as i32;
    arc_width = arc_width.max(4);
    let mut arc_gap = (base_radius as f32 * 0.08) as i32;
    arc_gap = arc_gap.max(3);

    // Hours arc (outermost)
    let hour_radius = base_radius.max(arc_width + arc_gap + 4);
    let hour_angle = (hours / 24.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        hour_radius,
        arc_width,
        hour_angle,
        Rgba8888::rgba(100, 140, 255, 255),
        Rgba8888::rgba(40, 60, 120, 100),
    );

    // Minutes arc (middle)
    let mut minute_radius = hour_radius - arc_width - arc_gap;
    if minute_radius <= arc_width {
        minute_radius = (hour_radius as f32 * 0.75) as i32;
    }
    minute_radius = minute_radius.max(arc_width + 6);
    let minute_angle = (minutes / 60.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        minute_radius,
        arc_width,
        minute_angle,
        Rgba8888::rgba(120, 255, 180, 255),
        Rgba8888::rgba(40, 100, 60, 100),
    );

    // Seconds arc (innermost)
    let mut second_radius = minute_radius - arc_width - arc_gap;
    if second_radius <= arc_width {
        second_radius = (minute_radius as f32 * 0.7) as i32;
    }
    second_radius = second_radius.max(arc_width + 4);
    let second_angle = (seconds / 60.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        second_radius,
        arc_width,
        second_angle,
        Rgba8888::rgba(255, 100, 120, 255),
        Rgba8888::rgba(120, 40, 60, 100),
    );

    (second_radius - arc_width - arc_gap).max(16)
}

fn draw_arc_ring(
    surface: &mut DrawingSurface,
    cx: i32,
    cy: i32,
    radius: i32,
    width: i32,
    angle: f32,
    active_color: Rgba8888,
    track_color: Rgba8888,
) {
    if radius <= 0 || width <= 0 {
        return;
    }

    // Background track (full circle)
    let mut track = ArcDsc::new(Point::new(cx, cy), radius, 0, 360);
    track.width = width;
    track.color = track_color;
    track.opa = OPA_COVER;
    draw_arc(surface, &track);

    // Active arc (progress)
    if angle > 0.1 {
        let mut active = ArcDsc::new(Point::new(cx, cy), radius, -90, angle as i32 - 90);
        active.width = width;
        active.color = active_color;
        active.opa = OPA_COVER;
        active.rounded = true;
        draw_arc(surface, &active);
    }
}

fn draw_center_info(
    surface: &mut DrawingSurface,
    cx: i32,
    cy: i32,
    max_radius: i32,
    hour_int: i32,
    minute_int: i32,
    second_int: i32,
    hour_progress: f32,
    minute_progress: f32,
    second_progress: f32,
    total_minutes: i32,
) {
    let content_radius = max_radius.max(24);

    // Background circle with subtle depth
    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(16, 16, 24, 220);
    bg.bg_opa = OPA_COVER;
    bg.bg_grad = Gradient::radial(
        Rgba8888::rgba(24, 24, 34, 220),
        Rgba8888::rgba(8, 8, 12, 200),
    );
    bg.radius = RADIUS_CIRCLE;
    bg.border_width = 2;
    bg.border_color = Rgba8888::rgba(80, 110, 180, 180);
    bg.border_opa = OPA_COVER;
    let bg_area = Area::new(
        cx - content_radius,
        cy - content_radius,
        cx + content_radius,
        cy + content_radius,
    );
    draw_rect(surface, &bg, &bg_area);

    draw_ticks(surface, cx, cy, content_radius);
    draw_hands(
        surface,
        cx,
        cy,
        content_radius,
        hour_progress,
        minute_progress,
        second_progress,
    );

    // Center hub
    let mut hub = RectDsc::new();
    hub.bg_color = Rgba8888::rgba(240, 240, 255, 255);
    hub.bg_opa = OPA_COVER;
    hub.radius = RADIUS_CIRCLE;
    let hub_area = Area::new(cx - 2, cy - 2, cx + 2, cy + 2);
    draw_rect(surface, &hub, &hub_area);

    let text_height = line_height();

    // Top label: elapsed minutes (mod 10000)
    let top_value = (total_minutes % 10_000).abs() as u32;
    let top_text = format!("{:04}", top_value);
    let top_width = measure_text(&top_text, 0);
    if top_width > 0 {
        let top_y = cy - content_radius + 6;
        let x1 = cx - top_width / 2;
        let top_area = Area::new(x1, top_y, x1 + top_width - 1, top_y + text_height - 1);
        let mut top_label = LabelDsc::new(top_text);
        top_label.color = Rgba8888::rgba(120, 180, 255, 200);
        draw_label(surface, &top_label, &top_area);
    }

    // Main digital time label (HH:MM)
    let time_text = format!("{:02}:{:02}", hour_int, minute_int);
    let time_width = measure_text(&time_text, 0);
    if time_width > 0 {
        let time_y = cy - text_height / 2;
        let x1 = cx - time_width / 2;
        let time_area = Area::new(x1, time_y, x1 + time_width - 1, time_y + text_height - 1);
        let mut time_label = LabelDsc::new(time_text);
        time_label.color = Rgba8888::rgba(235, 240, 255, 255);
        draw_label(surface, &time_label, &time_area);
    }

    // Seconds indicator beneath the main time
    let seconds_text = format!("{:02}", second_int);
    let seconds_width = measure_text(&seconds_text, 0);
    if seconds_width > 0 {
        let seconds_y = cy + text_height / 2 + 2;
        let x1 = cx - seconds_width / 2;
        let seconds_area = Area::new(
            x1,
            seconds_y,
            x1 + seconds_width - 1,
            seconds_y + text_height - 1,
        );
        let mut seconds_label = LabelDsc::new(seconds_text);
        seconds_label.color = Rgba8888::rgba(150, 210, 255, 220);
        draw_label(surface, &seconds_label, &seconds_area);
    }
}

fn draw_ticks(surface: &mut DrawingSurface, cx: i32, cy: i32, radius: i32) {
    if radius <= 10 {
        return;
    }

    let outer = radius - 2;
    let major_inner = outer - 8;
    let minor_inner = outer - 4;

    for idx in 0..60 {
        let angle = idx as f32 * 6.0 - 90.0;
        let end = angle_point(cx, cy, outer, angle);
        let inner_radius = if idx % 5 == 0 {
            major_inner.max(4)
        } else {
            minor_inner.max(4)
        };
        let start = angle_point(cx, cy, inner_radius, angle);

        let mut tick = LineDsc::new(start, end);
        tick.width = if idx % 5 == 0 { 2 } else { 1 };
        tick.width = tick.width.max(1);
        tick.round_start = true;
        tick.round_end = true;
        tick.color = if idx % 5 == 0 {
            Rgba8888::rgba(190, 200, 240, 200)
        } else {
            Rgba8888::rgba(120, 130, 170, 160)
        };
        draw_line(surface, &tick);
    }
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
