//! Simplified Analog Watch Application using new gfx API

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Shape, Line};
use crate::libs::gfx::Circle;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting watch app");
    loop {
        if !ctx.is_focused().await { Timer::after(Duration::from_millis(100)).await; continue; }
        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let w = surface.width() as i32;
            let h = surface.height() as i32;
            surface.fill_rect(0, 0, w, h, Rgba8888::rgba(20, 25, 35, 255));

            // Bezel
            Circle::new(w/2, h/2, (h.min(w) / 2 - 4))
                .stroke(2, Rgba8888::rgba(200, 200, 200, 255))
                .draw(surface);

            // Hands (static)
            Line::new(w/2, h/2, w/2, h/4)
                .stroke(4, Rgba8888::rgba(255, 215, 0, 255))
                .draw(surface);
            Line::new(w/2, h/2, 3*w/4, h/2)
                .stroke(3, Rgba8888::rgba(255, 255, 255, 255))
                .draw(surface);
        }).await;
        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(16)).await;
    }
}