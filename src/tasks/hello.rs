use core::fmt::Write;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::system::apps::app_context::AppContext;
use crate::system::services::battery::BATTERY_CHANNEL;

/// Battery app: draws battery percentage onto its canvas.
#[embassy_executor::task]
pub async fn hello_app(mut context: AppContext<'static>) {
    let receiver = BATTERY_CHANNEL.receiver();

    loop {
        // Only update if focused
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }


        context.canvas.clear();

        Rectangle::new(Point::zero(), Size::new(context.width(), context.height()))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(&mut context.canvas)
            .unwrap();


        // Just request redraw. No direct framebuffer or submit call.
        context.request_redraw().await;

        Timer::after(Duration::from_millis(100)).await;
    }
}
