#![no_std]

extern crate alloc;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::{Line, Shape, Text};
use crate::libs::gfx::{Arc, Circle, RoundedRect};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::task]
pub async fn gfx_bench_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context
            .draw(|surface: &mut DrawingSurface| {
                surface.fill_rect(
                    0,
                    0,
                    surface.width() as i32,
                    surface.height() as i32,
                    Rgba8888::rgba(0, 0, 0, 255),
                );
                info!("bench");
            })
            .await;
        Timer::after(Duration::from_millis(100)).await;
    }
}
