use core::fmt::Write;
use embassy_time::Timer;
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
pub async fn battery_task(mut context: AppContext<'static>) {
    let receiver = BATTERY_CHANNEL.receiver();

    loop {
        let percent = receiver.receive().await;

        context.canvas.clear();

        Rectangle::new(Point::zero(), Size::new(context.width(), context.height()))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut context.canvas)
            .unwrap();

        let mut text_buf = heapless::String::<32>::new();
        write!(text_buf, "Battery: {}%", percent).ok();

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        Text::new(&text_buf, Point::new(20, 28), style)
            .draw(&mut context.canvas)
            .unwrap();

        // ✅ Fixed: Canvas does not have `.submit()`, we submit via handle
        context.handle.submit(&context.canvas).await;

        Timer::after_millis(100).await;
    }
}
