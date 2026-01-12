//! Test app using new LVGL-compatible primitives

use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::rectangle::{RectDsc, draw_rect};
use rust_gfx::types::{Area, BorderSide, Gradient, GradDir, OPA_COVER};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn rect_app(ctx: AppContext) {
    info!("Starting rect app with new primitives");
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        ctx.draw(|surface: &mut DrawingSurface| {
            // Draw a fancy rectangle with gradient, border, and rounded corners
            let mut dsc = RectDsc::new();
            dsc.bg_color = Rgba8888::rgba(50, 100, 200, 255);
            dsc.bg_opa = OPA_COVER;
            dsc.bg_grad = Gradient::vertical(
                Rgba8888::rgba(100, 150, 255, 255),
                Rgba8888::rgba(50, 100, 200, 255)
            );
            dsc.radius = 10;
            dsc.border_color = Rgba8888::rgba(200, 200, 200, 255);
            dsc.border_width = 2;
            dsc.border_opa = OPA_COVER;
            dsc.border_side = BorderSide::FULL;

            let area = Area { x1: 20, y1: 20, x2: 80, y2: 80 };
            draw_rect(surface, &dsc, &area);
        })
        .await;

        Timer::after(Duration::from_millis(16)).await;
    }
}
