use embassy_time::{Duration, Timer};
use embedded_graphics::prelude::Primitive;
use embedded_graphics::primitives::PrimitiveStyleBuilder;
use embedded_graphics_core::prelude::*;
use embedded_graphics_core::primitives::Rectangle;
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Rgb332;

#[embassy_executor::task]
pub async fn ble_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|mut canvas| {
            let width = canvas.width() as i32;
            let height = canvas.height() as i32;

            // Define size of the rectangle
            let rect_size = 40;
            let x = (width - rect_size) / 2;
            let y = (height - rect_size) / 2;

            let rect = Rectangle::new(
                Point::new(x, y),
                Size::new(rect_size as u32, rect_size as u32),
            );

            // White fill style using RGB332
            let fill_style = PrimitiveStyleBuilder::new()
                .fill_color(Rgb332::new(7, 7, 3)) // max white in RGB332
                .build();

            rect.into_styled(fill_style)
                .draw(canvas)
                .unwrap();
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}
