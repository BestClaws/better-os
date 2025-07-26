use alloc::format;
use defmt::{dbg, debug, info};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::{Text}, mono_font::ascii::FONT_6X10, mono_font};
use core::fmt::Write;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, RoundedRectangle, StyledDrawable};
use embedded_graphics_core::pixelcolor::{Gray4, Rgb565};
use embedded_graphics_core::primitives::Rectangle;
use heapless::String;
use kolibri_embedded_gui::button::Button;
use kolibri_embedded_gui::label::Label;
use kolibri_embedded_gui::slider::Slider;
use kolibri_embedded_gui::style::{medsize_rgb565_style, Spacing, Style};
use kolibri_embedded_gui::ui::{Interaction, Ui};
use crate::libs::gfx::{draw_model, Model, Quaternion, RenderOptions, Vec3, parse_binary_stl};
use crate::system::app::app_context::AppContext;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::canvas::Canvas;

// Embed the binary STL file (place cube.stl in assets/ directory)
const STL_DATA: &[u8] = include_bytes!("../../src/assets/arrow2.stl");

#[embassy_executor::task]
pub async fn arrow_app(context: AppContext) {
    // Parse STL file
    let model = match parse_binary_stl(STL_DATA) {
        Ok(model) => model,
        Err(e) => {
            info!("STL parse error: {:?}", e);
            return; // Exit if parsing fails
        }
    };

    let mut rotation = Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
    let angle_increment = 1.0f32.to_radians();
    let mut last_time = Instant::now();
    let mut frame_count = 0u32;
    let mut fps = 0.0f32;

    let render_options = RenderOptions {
        fov_deg: 45.0,
        light_dir: Vec3(0.4, -0.4, -0.8),
        intensity_range: (0.2, 0.8),
    };

    let mut num: u32 = 0;
    let mut value = 0i16;

    let mut i = 0;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }





        context.draw(|canvas: &mut Canvas<Gray4>| {
            canvas.clear(Gray4::new(0x8)).unwrap();






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
                default_widget_height: 10,
                border_width: 2,
                highlight_border_width: 2,
                default_font: mono_font::iso_8859_10::FONT_9X18_BOLD,
                spacing: Spacing {
                    item_spacing: Size::new(2, 2),
                    button_padding: Size::new(6, 2),
                    default_padding: Size::new(6, 2),
                    window_border_padding: Size::new(10, 10),
                },
                corner_radius: 4,
            });

            if let Ok(HumanInputEvent::Touch(x, y)) = HUMAN_INPUT_CH.try_receive()  {
                info!("Touch: ({}, {})", x, y);

                ui.interact(Interaction::Release(Point::new(x, y)));

            }


            ui.clear_background().unwrap();
            ui.add(Label::new(format!("count: {num}").as_ref()));
            ui.add(Slider::new(&mut value, -100..=100));


            if ui.add_horizontal(Button::new("-")).clicked() {
                i -= 1;
            }

            ui.add_horizontal(Label::new(format!("{i}").as_ref()));
            if ui.add_horizontal(Button::new("+")).clicked() {
                i += 1;
            }






        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(120)).await;
        num += 1;
    }
}