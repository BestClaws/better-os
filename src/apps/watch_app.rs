//! Minimal analog watch face with a digital time badge.

use rust_gfx::color::Rgba8888;
use rust_gfx::rasterizer::Rasterizer;
use rust_gfx::primitives::{RectDsc, draw_rect, line::LineDsc, line::draw_line};
use rust_gfx::types::{Point, Area, Gradient, OPA_COVER, RADIUS_CIRCLE};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::format;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text as EgText;
use libm::{cosf, roundf, sinf};

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

            draw_background(surface, width, height);

            let cx = width / 2;
            let cy = height / 2;
            let radius = (height.min(width) as f32 * 0.42) as i32;

            let elapsed = Instant::now() - start;
            let secs_f = elapsed.as_micros() as f32 / 1_000_000.0;

            let seconds = secs_f % 60.0;
            let minutes = (secs_f / 60.0) % 60.0;
            let hours = (secs_f / 3600.0) % 12.0;

            let total_seconds = secs_f.max(0.0) as u32;
            let seconds_u = total_seconds % 60;
            let minutes_u = (total_seconds / 60) % 60;

            draw_face(surface, cx, cy, radius);
            draw_time_badge(surface, width, height, cx, cy, radius, minutes_u, seconds_u);
            draw_hands(surface, cx, cy, radius, hours, minutes, seconds);
        })
        .await;

        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

fn draw_background(surface: &mut DrawingSurface, width: i32, height: i32) {
    surface.fill_rect(0, 0, width, height, Rgba8888::rgba(6, 6, 10, 255));

    let glow_radius = (width.max(height) as f32 * 0.68) as i32;
    let mut rect = RectDsc::new();
    rect.bg_color = Rgba8888::rgba(44, 44, 52, 140);
    rect.bg_opa = OPA_COVER;
    rect.bg_grad = Gradient::radial(Rgba8888::rgba(44, 44, 52, 140), Rgba8888::rgba(0, 0, 0, 255));
    rect.radius = RADIUS_CIRCLE;
    let cx = width / 2;
    let cy = height / 2;
    let area = Area::new(cx - glow_radius, cy - glow_radius, glow_radius * 2, glow_radius * 2);
    draw_rect(surface, &rect, &area);
}

fn draw_face(surface: &mut DrawingSurface, cx: i32, cy: i32, radius: i32) {
    // Outer circle with stroke
    let mut outer = RectDsc::new();
    outer.bg_color = Rgba8888::rgba(78, 78, 86, 220);
    outer.bg_opa = OPA_COVER;
    outer.bg_grad = Gradient::radial(Rgba8888::rgba(78, 78, 86, 220), Rgba8888::rgba(8, 8, 12, 255));
    outer.radius = RADIUS_CIRCLE;
    outer.border_width = 2;
    outer.border_color = Rgba8888::rgba(110, 110, 118, 200);
    outer.border_opa = OPA_COVER;
    let r = radius + 6;
    let outer_area = Area::new(cx - r, cy - r, r * 2, r * 2);
    draw_rect(surface, &outer, &outer_area);

    // Inner circle
    let mut inner = RectDsc::new();
    inner.bg_color = Rgba8888::rgba(86, 86, 96, 255);
    inner.bg_opa = OPA_COVER;
    inner.bg_grad = Gradient::radial(Rgba8888::rgba(86, 86, 96, 255), Rgba8888::rgba(10, 10, 14, 255));
    inner.radius = RADIUS_CIRCLE;
    let inner_area = Area::new(cx - radius, cy - radius, radius * 2, radius * 2);
    draw_rect(surface, &inner, &inner_area);
}

fn draw_hands(
    surface: &mut DrawingSurface,
    cx: i32,
    cy: i32,
    radius: i32,
    hours: f32,
    minutes: f32,
    seconds: f32,
) {
    let tau = core::f32::consts::PI * 2.0;
    let up = -core::f32::consts::FRAC_PI_2;

    let hour_angle = up + tau * (hours / 12.0);
    let minute_angle = up + tau * (minutes / 60.0);
    let second_angle = up + tau * (seconds / 60.0);

    let hour_len = (radius as f32 * 0.55).max(18.0);
    let minute_len = (radius as f32 * 0.76).max(26.0);
    let second_len = (radius as f32 * 0.9).max(32.0);

    let hour_x = cx + roundf(hour_len * cosf(hour_angle)) as i32;
    let hour_y = cy + roundf(hour_len * sinf(hour_angle)) as i32;
    let mut hour_line = LineDsc::new(Point::new(cx, cy), Point::new(hour_x, hour_y));
    hour_line.width = 2;
    hour_line.color = Rgba8888::rgba(248, 248, 252, 255);
    hour_line.opa = OPA_COVER;
    draw_line(surface, &hour_line);

    let minute_x = cx + roundf(minute_len * cosf(minute_angle)) as i32;
    let minute_y = cy + roundf(minute_len * sinf(minute_angle)) as i32;
    let mut minute_line = LineDsc::new(Point::new(cx, cy), Point::new(minute_x, minute_y));
    minute_line.width = 2;
    minute_line.color = Rgba8888::rgba(248, 248, 252, 220);
    minute_line.opa = OPA_COVER;
    draw_line(surface, &minute_line);

    let second_x = cx + roundf(second_len * cosf(second_angle)) as i32;
    let second_y = cy + roundf(second_len * sinf(second_angle)) as i32;
    let second_tail_len = (radius as f32 * 0.12).max(4.0);
    let tail_x = cx - roundf(second_tail_len * cosf(second_angle)) as i32;
    let tail_y = cy - roundf(second_tail_len * sinf(second_angle)) as i32;
    let mut second_line = LineDsc::new(Point::new(tail_x, tail_y), Point::new(second_x, second_y));
    second_line.width = 1;
    second_line.color = Rgba8888::rgba(246, 64, 64, 255);
    second_line.opa = OPA_COVER;
    draw_line(surface, &second_line);

    // Center dot
    let mut center = RectDsc::new();
    center.bg_color = Rgba8888::rgba(246, 64, 64, 255);
    center.bg_opa = OPA_COVER;
    center.radius = RADIUS_CIRCLE;
    let center_area = Area::new(cx - 3, cy - 3, 6, 6);
    draw_rect(surface, &center, &center_area);
}

fn draw_time_badge(
    surface: &mut DrawingSurface,
    width: i32,
    height: i32,
    cx: i32,
    cy: i32,
    radius: i32,
    minutes: u32,
    seconds: u32,
) {
    let badge_width = 52;
    let badge_height = 24;
    let badge_x = cx - badge_width / 2;
    let face_bottom = cy + radius;
    let proposed_y = cy + radius / 2 - badge_height / 2;
    let max_y = height - badge_height - 8;
    let badge_y = proposed_y.min(max_y.max(0));

    // Glow behind badge
    let mut glow = RectDsc::new();
    glow.bg_color = Rgba8888::rgba(255, 232, 64, 40);
    glow.bg_opa = OPA_COVER;
    glow.bg_grad = Gradient::radial(Rgba8888::rgba(255, 232, 64, 40), Rgba8888::rgba(0, 0, 0, 0));
    glow.radius = 12;
    let glow_area = Area::new(badge_x - 4, badge_y - 4, badge_width + 8, badge_height + 8);
    draw_rect(surface, &glow, &glow_area);

    // Badge
    let mut badge = RectDsc::new();
    badge.bg_color = Rgba8888::rgba(0, 0, 0, 128);
    badge.bg_opa = OPA_COVER;
    badge.bg_grad = Gradient::horizontal(Rgba8888::rgba(0, 0, 0, 128), Rgba8888::rgba(0, 0, 0, 128));
    badge.radius = 10;
    badge.border_width = 2;
    badge.border_color = Rgba8888::rgba(252, 234, 78, 255);
    badge.border_opa = OPA_COVER;
    let badge_area = Area::new(badge_x, badge_y, badge_width, badge_height);
    draw_rect(surface, &badge, &badge_area);

    // TODO: Text rendering disabled - requires embedded-graphics
    // let text = format!("{:02}:{:02}", minutes, seconds);
    // let style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(246, 248, 255));
    // let mut target = SurfaceDrawTarget::new(surface);
    // let text_width = (text.len() as i32) * FONT_6X10.character_size.width as i32;
    // let text_height = FONT_6X10.character_size.height as i32;
    // let text_x = cx - text_width / 2;
    // let text_y = badge_y + (badge_height - text_height) / 2 + text_height - 2;
    // let _ = EgText::new(text.as_str(), Point::new(text_x, text_y), style).draw(&mut target);
}
