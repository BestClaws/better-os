#![allow(unused)]
use crate::libs::gfx::two_d::{Point, Size, Rgba8888, Canvas2D, PrimitiveRect, Paint, Stroke, Drawable};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use embassy_time::{Duration, Timer};
use micromath::F32Ext;

fn draw_rects(canvas: &mut Canvas2D) {
    // NEW FLUENT API - exactly what you wanted!
    PrimitiveRect::new(Point::new(25, 25), Size::new(25, 25))
        .fill(Paint::solid(Rgba8888::opaque(200, 60, 60)))
        .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 2.0))
        .corner_radius(5.0)
        .draw(canvas);
}

#[embassy_executor::task]
pub async fn rect_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        context.draw(|surface: &mut DrawingSurface| {
            use embassy_time::Instant;
            let draw_start = Instant::now();
            // Use Canvas2D fluent drawing over the DrawingSurface
            let mut c2d = Canvas2D::new(surface as &mut dyn crate::libs::gfx::two_d::Rasterizer);
            draw_rects(&mut c2d);
            
            let draw_duration = draw_start.elapsed();
            if draw_duration.as_millis() > 5 {
                defmt::info!("Rect draw: {}ms", draw_duration.as_millis());
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}
