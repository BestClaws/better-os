//! Simplified Analog Watch Application using new gfx API

use alloc::format;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Shape, Line, Text};
use crate::libs::gfx::{Circle, RoundedRect};
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use libm::{cosf, sinf, roundf};

#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting watch app");
    let start = Instant::now();
    loop {
        if !ctx.is_focused().await { Timer::after(Duration::from_millis(100)).await; continue; }
        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let width = surface.width() as i32;
            let height = surface.height() as i32;
            surface.fill_rect(0, 0, width, height, Rgba8888::rgba(20, 25, 35, 255));

            let cx = width / 2;
            let cy = height / 2;
            let bezel_r = (height.min(width) / 2 - 4);

            // Bezel
            Circle::new(cx, cy, bezel_r)
                .stroke(1, Rgba8888::rgba(200, 200, 200, 255))
                .fill_radial(Rgba8888::rgba(12, 13, 18, 150), Rgba8888::rgba(108, 19, 24, 150))
                .draw(surface);

            // Time since app start (monotonic). Drives the clock hands.
            let elapsed = Instant::now() - start;
            let micros = elapsed.as_micros();
            let secs_f = micros as f32 / 1_000_000.0;

            // Fractions for analog hands
            let s = secs_f % 60.0;
            let m = (secs_f / 60.0) % 60.0;      // includes seconds fraction
            let h = (secs_f / 3600.0) % 12.0;    // includes minutes fraction

            let tau = core::f32::consts::PI * 2.0;
            let up_offset = -core::f32::consts::FRAC_PI_2; // 12 o'clock at top

            let ang_s = up_offset + tau * (s / 60.0);
            let ang_m = up_offset + tau * (m / 60.0);
            let ang_h = up_offset + tau * (h / 12.0);

            let r_base = bezel_r as f32;
            let sec_len = (r_base - 6.0).max(0.0);
            let min_len = (r_base - 14.0).max(0.0);
            let hour_len = (r_base - 24.0).max(0.0);

            // Compute endpoints
            let sx = cx + roundf(sec_len * cosf(ang_s)) as i32;
            let sy = cy + roundf(sec_len * sinf(ang_s)) as i32;
            let mx = cx + roundf(min_len * cosf(ang_m)) as i32;
            let my = cy + roundf(min_len * sinf(ang_m)) as i32;
            let hx = cx + roundf(hour_len * cosf(ang_h)) as i32;
            let hy = cy + roundf(hour_len * sinf(ang_h)) as i32;

            // Draw hands
            Line::new(cx, cy, hx, hy)
                .stroke(1, Rgba8888::rgba(255, 215, 0, 255))
                .draw(surface);
            Line::new(cx, cy, mx, my)
                .stroke(1, Rgba8888::rgba(255, 255, 255, 255))
                .draw(surface);
            Line::new(cx, cy, sx, sy)
                .stroke(1, Rgba8888::rgba(255, 60, 60, 255))
                .draw(surface);

            // Center cap
            Circle::new(cx, cy, 2)
                .fill_solid( Rgba8888::rgba(255, 255, 255, 255))
                .draw(surface);


            RoundedRect::new(cx - 25, cy + 10, 52, 16, 5,5,5,5)
                .fill_linear_h(Rgba8888::rgba(255, 255, 255, 155), Rgba8888::rgba(255, 255, 0, 155))
                .stroke(1, Rgba8888::rgba(255, 255, 255, 255))
                .draw(surface);


            let time_str = format!("01:39");
            Text::new(cx - 20 as i32, cy + 15 as i32, &time_str)
                .color(Rgba8888::rgba(0, 0, 0, 255))
                .draw(surface);
        }).await;
        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(1)).await;
    }
}