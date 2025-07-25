use embassy_time::{Duration, Timer};
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_graphics_core::prelude::*;
use crate::system::app::app_context::AppContext;
use defmt::info;
use embedded_graphics::prelude::Primitive;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use crate::system::ui::canvas::Canvas;

#[embassy_executor::task]
pub async fn ble_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|mut canvas: &mut Canvas<Gray4>| {
            // let width = canvas.width() as i32;
            // let height = canvas.height() as i32;
            //
            // let start = Point::new(0, 0);
            // let end = Point::new(width - 1, height - 1);
            //
            // // Debug print of the start and end coordinates
            // info!("Drawing line from ({}, {}) to ({}, {})", start.x, start.y, end.x, end.y);
            //
            // let line = Line::new(start, end);
            // let style = PrimitiveStyle::with_stroke(Rgb565::new(7, 0, 0), 1); // Red stroke
            //
            // line.into_styled(style)
            //     .draw(canvas)
            //     .unwrap();
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}
