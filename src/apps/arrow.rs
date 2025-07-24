use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Text},
    mono_font::ascii::FONT_6X10,
};
use core::fmt::Write;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_graphics_core::pixelcolor::Rgb565;
use heapless::String;
use crate::libs::gfx::{draw_model, Model, Quaternion, RenderOptions, Vec3, parse_binary_stl};
use crate::system::app::app_context::AppContext;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

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

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }



        context.draw(|canvas| {
            canvas.clear();



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