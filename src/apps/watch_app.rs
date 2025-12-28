//! Minimal analog watch face with a digital time badge.

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Line, Shape};
use crate::libs::gfx::{Circle, RoundedRect, SurfaceDrawTarget};
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
    Circle::new(width / 2, height / 2, glow_radius)
        .fill_radial(
            Rgba8888::rgba(44, 44, 52, 140),
            Rgba8888::rgba(0, 0, 0, 255),
        )
        .draw(surface);
}

fn draw_face(surface: &mut DrawingSurface, cx: i32, cy: i32, radius: i32) {
    Circle::new(cx, cy, radius + 6)
        .fill_radial(
            Rgba8888::rgba(78, 78, 86, 220),
            Rgba8888::rgba(8, 8, 12, 255),
        )
        .stroke(2, Rgba8888::rgba(110, 110, 118, 200))
        .draw(surface);

    Circle::new(cx, cy, radius)
        .fill_radial(
            Rgba8888::rgba(86, 86, 96, 255),
            Rgba8888::rgba(10, 10, 14, 255),
        )
        .draw(surface);
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
    Line::new(cx, cy, hour_x, hour_y)
        .stroke(2, Rgba8888::rgba(248, 248, 252, 255))
        .draw(surface);

    let minute_x = cx + roundf(minute_len * cosf(minute_angle)) as i32;
    let minute_y = cy + roundf(minute_len * sinf(minute_angle)) as i32;
    Line::new(cx, cy, minute_x, minute_y)
        .stroke(2, Rgba8888::rgba(248, 248, 252, 220))
        .draw(surface);

    let second_x = cx + roundf(second_len * cosf(second_angle)) as i32;
    let second_y = cy + roundf(second_len * sinf(second_angle)) as i32;
    let second_tail_len = (radius as f32 * 0.12).max(4.0);
    let tail_x = cx - roundf(second_tail_len * cosf(second_angle)) as i32;
    let tail_y = cy - roundf(second_tail_len * sinf(second_angle)) as i32;
    Line::new(tail_x, tail_y, second_x, second_y)
        .stroke(1, Rgba8888::rgba(246, 64, 64, 255))
        .draw(surface);

    Circle::new(cx, cy, 3)
        .fill_solid(Rgba8888::rgba(246, 64, 64, 255))
        .draw(surface);
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

    RoundedRect::new(
        badge_x - 4,
        badge_y - 4,
        badge_width + 8,
        badge_height + 8,
        12,
        12,
        12,
        12,
    )
    .fill_radial(Rgba8888::rgba(255, 232, 64, 40), Rgba8888::rgba(0, 0, 0, 0))
    .draw(surface);

    RoundedRect::new(badge_x, badge_y, badge_width, badge_height, 10, 10, 10, 10)
        .fill_linear_h(Rgba8888::rgba(0, 0, 0, 128), Rgba8888::rgba(0, 0, 0, 128))
        .stroke(2, Rgba8888::rgba(252, 234, 78, 255))
        .draw(surface);

    let text = format!("{:02}:{:02}", minutes, seconds);
    let style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(246, 248, 255));
    let mut target = SurfaceDrawTarget::new(surface);
    let text_width = (text.len() as i32) * FONT_6X10.character_size.width as i32;
    let text_height = FONT_6X10.character_size.height as i32;
    let text_x = cx - text_width / 2;
    let text_y = badge_y + (badge_height - text_height) / 2 + text_height - 2;
    let _ = EgText::new(text.as_str(), Point::new(text_x, text_y), style).draw(&mut target);
}
