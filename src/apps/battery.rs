// Allow unused code for prototyping
#![allow(unused)]
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Triangle},
};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::battery_srv::BATTERY_CHANNEL;
use crate::system::services::gyro_accel_srv::ORIENTATION_CHANNEL;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    let receiver = BATTERY_CHANNEL.receiver();

    loop {
        let direction = receiver.receive().await;

        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let percent = receiver.receive().await;

        context.draw(|mut canvas: &mut Canvas<Gray4>| {

            canvas.clear(Gray4::WHITE);
            //
            // let mut text_buf = heapless::String::<32>::new();
            // write!(text_buf, "Battery: {}%", percent).ok();
            //
            // let style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(255, 255, 255));
            // Text::new(&text_buf, Point::new(20, 28), style)
            //     .draw(canvas)
            //     .unwrap();
            //



        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }

}

