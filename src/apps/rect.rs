//! Test app using new LVGL-compatible primitives

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Timer};
use rust_gfx::color::Rgba8888;
use rust_gfx::fluent::{Axis, FillPlan, GradientBuilder, GradientStop, Radius, Rect, StrokePlan};
use rust_gfx::types::Area;

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
            let area = Area {
                x1: 20,
                y1: 20,
                x2: 80,
                y2: 80,
            };
            let gradient = GradientBuilder::linear()
                .axis(Axis::Vertical)
                .stops([
                    GradientStop::new(0.0, Rgba8888::rgba(r2, g2, b2, 255)),
                    GradientStop::new(1.0, Rgba8888::rgba(r, g, b, 255)),
                ])
                .finish();

            Rect::new()
                .area(area)
                .radius(Radius::uniform(10))
                .fill(FillPlan::Gradient { gradient })
                .stroke(StrokePlan::solid(2, Rgba8888::rgba(200, 200, 200, 255)))
                .finish()
                .draw(surface);
        })
        .await;

        hue = (hue + 3) % 360;
        Timer::after(Duration::from_millis(16)).await;
    }
}
