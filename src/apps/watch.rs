use core::f32::consts::PI;
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use crate::libs::gfx::two_d::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, LinearGradient, RadialGradient,
                              draw_line_aa, draw_arc_aa, fill_rounded_rect, fill_rect_linear_gradient, fill_rect_radial_gradient};
use micromath::F32Ext;

const TAU: f32 = 2.0 * PI;

#[embassy_executor::task]
pub async fn watch_app(context: AppContext) {
    info!("Watch app started");
    let start_time = Instant::now();

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let elapsed_ms = start_time.elapsed().as_millis() as f32;
        let t = elapsed_ms / 1000.0; // Time in seconds
        let hour = ((t / 3600.0) % 12.0) as i32;
        let minute = ((t / 60.0) % 60.0) as i32;
        let second = (t % 60.0) as i32;

        context.draw(|canvas: &mut Canvas| {
            let w = canvas.width();
            let h = canvas.height();
            // Background with radial gradient
            let center = GPoint::new(canvas.width() as i32 / 2, canvas.height() as i32 / 2);
            let radius = canvas.height() as i32 / 2 - 20;
            let grad_bg = RadialGradient {
                center,
                radius: radius as u32,
                inner_color: Rgb565::from_rgb(0, 0, 0),
                outer_color: Rgb565::from_rgb(15, 15, 20),
            };
            fill_rect_radial_gradient(canvas, GRect::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32)), &grad_bg);

            // Outer ring with linear gradient
            let outer_rect = GRect::new(GPoint::new(0, 0), GSize::new((canvas.width()) as u32, (canvas.height()) as u32));
            let grad_outer = LinearGradient {
                start: GPoint::new(10, 10),
                end: GPoint::new((canvas.width() - 10) as i32, (canvas.height() - 10) as i32),
                start_color: Rgb565::from_rgb(50, 50, 60),
                end_color: Rgb565::from_rgb(0, 0, 0),
            };
            fill_rect_linear_gradient(canvas, outer_rect, &grad_outer);

            // Hour markers with arcs
            for i in 0..12 {
                let angle = (i as f32 / 12.0) * TAU;
                let x = center.x + (radius as f32 * 0.9 * angle.cos()) as i32;
                let y = center.y + (radius as f32 * 0.9 * angle.sin()) as i32;
                let sweep = 0.1 + 0.05 * (t * 0.5 + i as f32).sin();
                draw_arc_aa(canvas, center, radius, angle - sweep, angle + sweep, Rgb565::from_rgb(200, 200, 200));
            }

            // Center rounded rect with gradient
            let inner_w = (canvas.width() / 3).max(40) as u32;
            let inner_h = (canvas.height() / 3).max(40) as u32;
            let inner_x = center.x - (inner_w / 2) as i32;
            let inner_y = center.y - (inner_h / 2) as i32;
            let inner_r = (inner_h / 4).max(4) as i32;
            fill_rounded_rect(canvas, GRect::new(GPoint::new(inner_x, inner_y), GSize::new(inner_w, inner_h)), inner_r, Rgb565::from_rgb(50, 180, 90));

            // Hands with dynamic styling
            let hour_angle = (hour as f32 + minute as f32 / 60.0) * 30.0 * PI / 180.0;
            let minute_angle = minute as f32 * 6.0 * PI / 180.0;
            let second_angle = second as f32 * 6.0 * PI / 180.0;

            let hour_x = center.x + ((radius as f32 * 0.5 * hour_angle.cos()) as i32);
            let hour_y = center.y + ((radius as f32 * 0.5 * hour_angle.sin()) as i32);
            draw_line_aa(canvas, center, GPoint::new(hour_x, hour_y), Rgb565::from_rgb(255, 180, 0));

            let minute_x = center.x + ((radius as f32 * 0.7 * minute_angle.cos()) as i32);
            let minute_y = center.y + ((radius as f32 * 0.7 * minute_angle.sin()) as i32);
            draw_line_aa(canvas, center, GPoint::new(minute_x, minute_y), Rgb565::from_rgb(0, 200, 255));

            let second_x = center.x + ((radius as f32 * 0.9 * second_angle.cos()) as i32);
            let second_y = center.y + ((radius as f32 * 0.9 * second_angle.sin()) as i32);
            draw_line_aa(canvas, center, GPoint::new(second_x, second_y), Rgb565::from_rgb(120, 255, 120));
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1000)).await; // Update every second
    }
}