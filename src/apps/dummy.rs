// Allow unused code for prototyping
#![allow(unused)]

use crate::system::app::app_context::AppContext;
use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::Canvas;
use embassy_time::{Duration, Timer};
use embedded_graphics::primitives::{Circle, StyledDrawable};
use embedded_graphics::{prelude::*, primitives::PrimitiveStyle};
use embedded_graphics_core::pixelcolor::Gray4;

#[embassy_executor::task]
pub async fn dummy_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let input = context.poll_input().await;

        if input.is_none() {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let input = input.unwrap();

        let HumanInputEvent::Touch(tx, ty) = input else {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        };

        context
            .draw(|mut canvas: &mut Canvas<Gray4>| {

                let style = PrimitiveStyle::with_fill(Gray4::WHITE);
                Circle::with_center(Point::new(tx, ty), 10)
                .into_styled(style)
                .draw(canvas)
                .unwrap();
            })
            .await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}
