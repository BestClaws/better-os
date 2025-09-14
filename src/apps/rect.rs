#![allow(unused)]
use crate::libs::gfx::two_d::{Point as GPoint, Rect as GRect, Rgb565, Size as GSize, Canvas2D};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use embassy_time::{Duration, Timer};
use micromath::F32Ext;

fn draw_rects(c2d: &mut Canvas2D) {
    let mut d = crate::libs::gfx::two_d::draw::Draw::new(c2d.raster_mut());
    d.rect(GRect::new(GPoint::new(25, 25), GSize::new(25 as u32, 25 as u32)))
        .fill_color(Rgb565::from_rgb(200, 60, 60))
        .draw();
}

#[embassy_executor::task]
pub async fn rect_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        context.draw(|surface: &mut DrawingSurface| {
            // Use Canvas2D fluent drawing over the DrawingSurface
            let mut c2d = Canvas2D::new(surface as &mut dyn crate::libs::gfx::two_d::Rasterizer);
            draw_rects(&mut c2d);
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}
