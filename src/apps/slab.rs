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
use heapless::String;
use crate::libs::gfx::{draw_model, Model, Quaternion, RenderOptions, Vec3, parse_binary_stl};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Rgb332;

// Embed the binary STL file (place cube.stl in assets/ directory)
const STL_DATA: &[u8] = include_bytes!("../../src/assets/arrow2.stl");

#[embassy_executor::task]
pub async fn cube_app(context: AppContext) {
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

        let rot_x = Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), angle_increment);
        let rot_y = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), angle_increment);
        let rot_z = Quaternion::from_axis_angle(Vec3(0.0, 0.0, 1.0), angle_increment);
        rotation = rot_z.mul(rot_y.mul(rot_x.mul(rotation)));

        frame_count += 1;
        let current_time = Instant::now();
        let elapsed = current_time - last_time;
        if elapsed.as_millis() >= 1000 {
            fps = frame_count as f32 * 1000.0 / elapsed.as_millis() as f32;
            frame_count = 0;
            last_time = current_time;
        }

        context.draw(|canvas| {
            canvas.clear();
            if let Err(e) = draw_model(
                canvas,
                &model,
                Vec3(0.0, 0.0, 1.4),
                rotation,
                canvas.width(),
                canvas.height(),
                &render_options,
            ) {
                info!("Draw error: {:?}", e);
            }

            let mut text_buf = String::<32>::new();
            write!(text_buf, "FPS: {:.1}", fps).ok();
            let style = MonoTextStyle::new(&FONT_6X10, Rgb332::new(7, 0, 0));
            Text::new(&text_buf, Point::new(0, 10), style)
                .draw(canvas)
                .unwrap();

        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}