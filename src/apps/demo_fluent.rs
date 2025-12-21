#![no_std]

use embassy_time::{Duration, Instant, Timer};
use defmt::info;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Arc, Circle, RoundedRect};
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::shapes::Shape;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use micromath::F32Ext;
use crate::util::math::primitives::{Point, Size};

// Easing functions for smooth animations
fn ease_in_out_sine(t: f32) -> f32 {
    -(t * core::f32::consts::PI).cos() / 2.0 + 0.5
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn smooth_lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * ease_in_out_sine(t)
}

#[embassy_executor::task]
pub async fn demo_fluent_app(context: AppContext) {
    info!("Comprehensive Fluent GFX API Demo Started - Showcasing All Features!");
    let start = Instant::now();
    
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let t = start.elapsed().as_micros() as f32 / 1_000_000.0;
        
        // Cycle through different demos every 4 seconds for smoother transitions
        let demo_phase = ((t / 4.0) as u32) % 7;
        let raw_demo_t = (t % 4.0) / 4.0; // 0.0 to 1.0 within each demo
        let demo_t = ease_in_out_cubic(raw_demo_t); // Apply easing for smoother transitions
        
        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            let w = surface.width() as i32;
            let h = surface.height() as i32;
            surface.fill_rect(0, 0, w, h, Rgba8888::rgba(20, 25, 35, 255));

            // Simple fluent demo using new shapes
            Circle::new(w / 3, h / 2, 40)
                .fill_solid(Rgba8888::rgba(100, 150, 255, 255))
                .stroke(2, Rgba8888::rgba(255, 255, 255, 255))
                .draw(surface);

            crate::libs::gfx::shapes::Line::new(w / 4, h / 4, 3 * w / 4, 3 * h / 4)
                .stroke(4, Rgba8888::rgba(255, 100, 100, 255))
                .draw(surface);

            Arc::new(2 * w / 3, h / 2, 35, 0, 180)
                .stroke(3, Rgba8888::rgba(100, 255, 150, 255))
                .draw(surface);

            RoundedRect::new(w / 2 - 40, h / 6, 80, 50, 10, 10, 10, 10)
                .fill_linear_h(Rgba8888::rgba(255, 120, 180, 180), Rgba8888::rgba(120, 180, 255, 200))
                .stroke(2, Rgba8888::rgba(200, 200, 200, 255))
                .draw(surface);

            let draw_time = draw_start.elapsed().as_micros();
            info!("Fluent demo render time: {} μs", draw_time);
        }).await;
        
            Timer::after(Duration::from_millis(8)).await; // Increased refresh rate for smoother animations
    }
}

