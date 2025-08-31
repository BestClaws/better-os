// Allow unused code for prototyping
#![allow(unused)]

use alloc::format;
use alloc::string::ToString;
use core::fmt::Write;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{mono_font, pixelcolor::BinaryColor, prelude::*, primitives::{Line, PrimitiveStyle, Triangle}};
use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_6X10};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use kolibri_embedded_gui::button::Button;
use kolibri_embedded_gui::label::Label;
use kolibri_embedded_gui::style::{Spacing, Style};
use kolibri_embedded_gui::ui::{Interaction, Ui};
use micromath::F32Ext;
use crate::system::app::app_context::AppContext;
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::services::vibrator_srv::VIBRATION_SIG;
use crate::system::ui::canvas::{Canvas};
use crate::util::math::primitives::Vec3;

#[embassy_executor::task]
pub async fn dummy_app(context: AppContext) {

    let mut i = 0;
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw(|mut canvas: &mut Canvas<Rgb565>| {

            let mut ui = Ui::new(canvas, Rectangle::new(Point::new(0, 0), Size::new(canvas.width(), canvas.height())), Style {
                background_color: Rgb565::new(8, 0, 0), // pretty dark gray
                item_background_color:Rgb565::new(0, 8, 0), // darker gray
                highlight_item_background_color: Rgb565::new(0, 0, 8),
                border_color: Rgb565::new(16, 0, 0),
                highlight_border_color: Rgb565::new(0, 16, 0),
                primary_color: Rgb565::new(32, 0, 0),
                secondary_color: Rgb565::new(0, 32, 0),
                icon_color: Rgb565::new(0, 0, 32),
                text_color: Rgb565::new(0, 12, 12),
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

            if let Ok(HumanInputEvent::Touch(x, y)) = HUMAN_INPUT_CH.try_receive()  {

                ui.interact(Interaction::Click(Point::new(x, y)));
                ui.interact(Interaction::Release(Point::new(x, y)));

            }


            ui.clear_background().unwrap();
            ui.add(Label::new("Basic").with_font(FONT_4X6));

            if ui.add_horizontal(Button::new("-")).clicked() {
                i -= 1;
            }

            ui.add_horizontal(Label::new(format!("{i}").as_ref()));
            if ui.add_horizontal(Button::new("+")).clicked() {
                i += 1;
            }


            ui.finalize().unwrap();





        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }

}

