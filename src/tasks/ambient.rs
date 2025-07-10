use embassy_time::Timer;
use crate::system::services::ambient_sensor::AMBIENT_CHANNEL;
use crate::system::ui::canvas::Canvas;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use core::fmt::Write;

#[embassy_executor::task]
pub async fn ambient_task() {
    // let receiver = AMBIENT_CHANNEL.receiver();
    //
    // loop {
    //     let percent = receiver.receive().await;
    //
    //     let canvas = ctx.canvas();
    //     canvas.clear();
    //     draw_ui(canvas, percent);
    //
    //     Timer::after_millis(100).await;
    // }
}

fn draw_ui(canvas: &mut Canvas<'_>, percent: u8) {
    Rectangle::new(Point::zero(), canvas.size())
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(canvas)
        .unwrap();

    let mut text_buf = heapless::String::<32>::new();
    let _ = write!(text_buf, "Ambient Light: {}%", percent);

    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::new(&text_buf, Point::new(16, 28), style)
        .draw(canvas)
        .unwrap();
}
