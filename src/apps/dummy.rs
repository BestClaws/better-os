// Allow unused code for prototyping
#![allow(unused)]

use alloc::string::ToString;
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{mono_font, pixelcolor::BinaryColor, prelude::*, primitives::{Line, PrimitiveStyle, Triangle}};
use embedded_graphics::mono_font::ascii::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use kolibri_embedded_gui::label::Label;
use kolibri_embedded_gui::style::{Spacing, Style};
use kolibri_embedded_gui::ui::Ui;
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::vibrator_srv::VIBRATION_SIG;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

#[embassy_executor::task]
pub async fn dummy_app(context: AppContext) {

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|mut canvas: &mut Canvas<Gray4>| {

            let mut ui = Ui::new(canvas, Rectangle::new(Point::new(0, 0), Size::new(canvas.width(), canvas.height())), Style {
                background_color: Gray4::new(0xf), // pretty dark gray
                item_background_color: Gray4::new(0xd), // darker gray
                highlight_item_background_color: Gray4::new(0xb),
                border_color: Gray4::new(0x0),
                highlight_border_color: Gray4::new(0x2),
                primary_color: Gray4::new(0xc),
                secondary_color: Gray4::new(0x3),
                icon_color: Gray4::new(0x0),
                text_color: Gray4::new(0x0),
                default_widget_height: 8,
                border_width: 2,
                highlight_border_width: 2,
                default_font: mono_font::iso_8859_10::FONT_9X18_BOLD,
                spacing: Spacing {
                    item_spacing: Size::new(2, 2),
                    button_padding: Size::new(6, 2),
                    default_padding: Size::new(10, 6),
                    window_border_padding: Size::new(10, 10),
                },
                corner_radius: 4,
            });

            // ui.clear_background().unwrap();
            ui.add(Label::new("DUMMY".to_string().as_ref()));




        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }

}

