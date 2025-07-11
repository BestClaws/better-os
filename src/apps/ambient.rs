use core::fmt::Write;
use embassy_time::{Timer, Duration};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::system::app::app_context::AppContext;
use crate::system::services::ambient_srv::AMBIENT_CHANNEL;

#[embassy_executor::task]
pub async fn ambience_app(mut context: AppContext<'static>) {
    let receiver = AMBIENT_CHANNEL.receiver();

    loop {
        // Only update if focused
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let percent = receiver.receive().await;

        context.canvas.clear();

        Rectangle::new(Point::zero(), Size::new(context.width(), context.height()))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut context.canvas)
            .unwrap();

        let mut text_buf = heapless::String::<32>::new();
        write!(text_buf, "ambient: {}%", percent).ok();

        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        Text::new(&text_buf, Point::new(20, 28), style)
            .draw(&mut context.canvas)
            .unwrap();

        context.request_redraw().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}
