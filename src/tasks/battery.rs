use alloc::boxed::Box;

use embassy_sync::semaphore::Semaphore;
use embassy_time::Timer;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{Point, Primitive, Size};
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use crate::{
    system::resources::framebuffer::{request_framebuffer, FB_SEMAPHORE, FRAME_CHANNEL, SubmitFrame},
};

#[embassy_executor::task]
pub async fn battery() {
    let receiver = BATTERY_CHANNEL.receiver();

    loop {
        let percent = receiver.receive().await;

        FB_SEMAPHORE.acquire(1).await.unwrap();

        if let Some(fb) = request_framebuffer() {
            draw_ui(fb.buf, percent);

            FRAME_CHANNEL
                .sender()
                .send(SubmitFrame {
                    id: fb.id,
                    app_id: 2,
                })
                .await;

            core::mem::forget(fb); // compositor owns and drops it
        }

        Timer::after_millis(100).await;
    }
}

use core::fmt::Write;
use crate::system::resources::framebuffer::BitPackedFramebuffer;
use crate::system::services::battery::BATTERY_CHANNEL;

pub fn draw_ui(buf: &mut [u8; 1024], percent: u8) {
    let mut fb = BitPackedFramebuffer {
        buf,
        width: 128,
        height: 64,
    };

    // Border
    Rectangle::new(Point::zero(), Size::new(128, 64))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(&mut fb)
        .unwrap();

    // Label
    let mut text_buf = heapless::String::<32>::new();
    let _ = write!(text_buf, "Battery: {}%", percent);

    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    Text::new(&text_buf, Point::new(20, 28), text_style)
        .draw(&mut fb)
        .unwrap();
}

