#![allow(unused)]
use crate::libs::gfx::two_d::{
    draw_rect_outline_aa, fill_rect, Point as GPoint, Rect as GRect, Rgb565, Size as GSize,
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use embassy_time::{Duration, Timer};
use micromath::F32Ext;

fn draw_rects(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(15, 15, 20));


    fill_rect(
        canvas,
        GRect::new(GPoint::new(25, 25), GSize::new(25 as u32, 25 as u32)),
        Rgb565::from_rgb(200, 60, 60),
    );

}

#[embassy_executor::task]
pub async fn rect_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        context
            .draw(|canvas: &mut Canvas| {
                draw_rects(canvas);
            })
            .await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}
