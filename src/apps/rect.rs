//! Simplified Analog Watch Application using new gfx API

use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Line, Shape};
use crate::libs::gfx::Circle;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use libm::{cosf, roundf, sinf};

#[embassy_executor::task]
pub async fn rect_app(ctx: AppContext) {
    info!("Starting watch app");
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        ctx.draw(|surface: &mut DrawingSurface| {
            Circle::new(50, 50, 20)
                .stroke(2, Rgba8888::rgba(200, 200, 200, 255))
                .draw(surface);
        })
        .await;

        Timer::after(Duration::from_millis(16)).await;
    }
}
