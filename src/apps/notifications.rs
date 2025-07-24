// Allow unused code for prototyping
#![allow(unused)]
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle},
};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::Rgb565;
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::vibrator_srv::VIBRATION_SIG;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

#[embassy_executor::task]
pub async fn notifications_app(context: AppContext) {

    let mut last_here = Instant::now();

    loop {

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        VIBRATION_SIG.signal(Duration::from_millis(100));

        context.draw(|mut canvas| {

            canvas.clear();

            let now = Instant::now();
            let dt = now - last_here;
            last_here = now;

            let mut text_buf = heapless::String::<32>::new();
            write!(text_buf, "BLE.\nlast: {}",dt.as_micros()).ok();

            let style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(255, 255, 255));
            Text::new(&text_buf, Point::new(20, 28), style)
                .draw(canvas)
                .unwrap();


        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1000)).await;
    }

}

