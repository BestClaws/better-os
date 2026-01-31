//! Test app using new LVGL-compatible primitives

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Timer};
use gfx::colors::Color;
use gfx::primitives::rectangle::{draw_rect, RectDsc};
use gfx::types::{Area, BorderSide, GradDir, Gradient, OPA_COVER};

#[embassy_executor::task]
pub async fn rect_app(ctx: AppContext) {
    info!("Starting rect app with new primitives");
    let mut hue: u32 = 0;
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        // Calculate color based on hue (cycling through colors)
        let r = ((((hue * 6) % 360) as f32 / 360.0 * 255.0) as u8).wrapping_add(50);
        let g = ((((hue * 4) % 360) as f32 / 360.0 * 255.0) as u8).wrapping_add(50);
        let b = ((((hue * 2) % 360) as f32 / 360.0 * 255.0) as u8).wrapping_add(100);

        let r2 = r.wrapping_add(50);
        let g2 = g.wrapping_add(50);
        let b2 = b.wrapping_add(50);

        ctx.draw(move |surface: &mut DrawingSurface| {
            // Draw a fancy rectangle with gradient, border, and rounded corners
            let mut dsc = RectDsc::new();
            dsc.bg_color = Color::rgba(r, g, b, 255);
            dsc.bg_opa = OPA_COVER;
            dsc.bg_grad = Gradient::vertical(
                Color::rgba(r2, g2, b2, 255),
                Color::rgba(r, g, b, 255),
            );
            dsc.radius = 10;
            dsc.border_color = Color::rgba(200, 200, 200, 255);
            dsc.border_width = 2;
            dsc.border_opa = OPA_COVER;
            dsc.border_side = BorderSide::FULL;

            let area = Area {
                x1: 20,
                y1: 20,
                x2: 80,
                y2: 80,
            };
            draw_rect(surface, &dsc, &area);
        })
        .await;

        hue = (hue + 3) % 360;
        Timer::after(Duration::from_millis(16)).await;
    }
}
