use alloc::format;
use defmt::{dbg, debug, info};
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{pixelcolor::BinaryColor, prelude::*, text::{Text}, mono_font::ascii::FONT_6X10, mono_font};
use core::fmt::Write;
use embedded_graphics::mono_font::{ascii, MonoTextStyle};
use embedded_graphics::mono_font::ascii::FONT_4X6;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::primitives::Rectangle;
use heapless::String;
use kolibri_embedded_gui::button::Button;
use kolibri_embedded_gui::label::Label;
use kolibri_embedded_gui::style::{medsize_rgb565_style, Spacing, Style};
use kolibri_embedded_gui::ui::{Interaction, Ui};
use crate::libs::gfx::{draw_model, Model, Quaternion, RenderOptions, Vec3, parse_binary_stl};
use crate::system::app::app_context::AppContext;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};

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

    let mut i: i32 = 0;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }




        context.draw(|canvas| {
            // canvas.clear();


            let mut ui = Ui::new(canvas, Rectangle::new(Point::new(0, 0), Size::new(canvas.width(), canvas.height())), Style {
                background_color: Rgb565::new(0x14, 0x8, 0x4), // pretty dark gray
                item_background_color: Rgb565::new(0xf, 0x4, 0x2), // darker gray
                highlight_item_background_color: Rgb565::new(0x1, 0x2, 0x1),
                border_color: Rgb565::WHITE,
                highlight_border_color: Rgb565::WHITE,
                primary_color: Rgb565::CSS_DARK_CYAN,
                secondary_color: Rgb565::YELLOW,
                icon_color: Rgb565::WHITE,
                text_color: Rgb565::WHITE,
                default_widget_height: 4,
                border_width: 0,
                highlight_border_width: 1,
                default_font: mono_font::iso_8859_10::FONT_4X6,
                spacing: Spacing {
                    item_spacing: Size::new(1, 1),
                    button_padding: Size::new(1, 1),
                    default_padding: Size::new(1, 1),
                    window_border_padding: Size::new(1, 1),
                },
            });

            if let Ok(HumanInputEvent::Touch(x, y)) = HUMAN_INPUT_CH.try_receive()  {
                info!("Touch: ({}, {})", x, y);

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




            // Draw a red diagonal line from (0,0) to (239,319)
            let line_style = PrimitiveStyle::with_stroke(Rgb565::new(31, 0, 0), 1);
            Line::new(Point::new(0, 0), Point::new(FRAME_BUFFER_WIDTH as i32, FRAME_BUFFER_HEIGHT as i32))
                .into_styled(line_style)
                .draw(canvas)
                .unwrap();



        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}