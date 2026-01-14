//! Gray test app - cycles through different gray levels

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Timer};
use rust_gfx::color::Rgba8888;

#[embassy_executor::task]
pub async fn gray_test_app(ctx: AppContext) {
    info!("Gray Test: Starting");

    // Wait a moment to ensure window is ready
    Timer::after(Duration::from_millis(100)).await;

    let gray_levels = [
        (0, "Black (0)"),
        (255, "White (255)"),
        (128, "50% Gray (128)"),
        (64, "25% Gray (64)"),
        (192, "75% Gray (192)"),
        (32, "12.5% Gray (32)"),
        (224, "87.5% Gray (224)"),
        (96, "37.5% Gray (96)"),
        (160, "62.5% Gray (160)"),
    ];

    for (gray_val, name) in gray_levels.iter().cycle() {
        info!(
            "Gray Test: Drawing {} - RGB({},{},{})",
            name, gray_val, gray_val, gray_val
        );

        let color = Rgba8888::rgba(*gray_val, *gray_val, *gray_val, 255);

        ctx.draw(|surface: &mut DrawingSurface| {
            surface.clear(color);
        })
        .await;

        // Hold each level for 2 seconds
        Timer::after(Duration::from_secs(2)).await;
    }
}
