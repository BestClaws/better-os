// 3D rendering removed - legacy API no longer available
/*
use rust_gfx::color::Rgba8888;
use rust_gfx::{draw_model, parse_binary_stl, Quaternion, RenderOptions, Vec3};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};

const STL_DATA: &[u8] = include_bytes!("../assets/geofix.stl");

#[embassy_executor::task]
pub async fn arrow_app(context: AppContext) {
    let model = match parse_binary_stl(STL_DATA) {
        Ok(model) => model,
        Err(e) => {
            defmt::error!("3D arrow STL parse error");
            return;
        }
    };

    let mut rotation = Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let angle_increment = 8.0f32.to_radians();
    let mut last_time = Instant::now();
    let mut frame_count = 0u32;
    let mut fps = 0.0f32;

    let render_options = RenderOptions {
        fov_deg: 45.0,
        light_dir: Vec3(0.4, -0.4, -0.8),
        intensity_range: (0.2, 0.8),
        enable_backface_culling: true,
        enable_zbuffer: true,
        enable_lighting: true,
        enable_depth_sorting: true,
        enable_near_clipping: true,
        enable_frustum_clipping: true,
        enable_wireframe: false,
        enable_shading: true,
        enable_antialiasing: true,
        antialiasing_factor: 1,
        edge_only_antialiasing: true,
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
            info!("arrow fps: {}", fps);
        }

        context
            .draw(|surface: &mut DrawingSurface| {
                surface.clear(Rgba8888::BLACK);

                let frame_start = Instant::now();
                draw_model(
                    surface,
                    &model,
                    Vec3(0.0, 0.0, 3.0),
                    rotation,
                    &render_options,
                );
                info!(
                    "arrow draw time: {} ms",
                    (Instant::now() - frame_start).as_millis()
                );
            })
            .await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}
*/

// Stub implementation while 3D rendering is not available
use crate::system::app::app_context::AppContext;

#[embassy_executor::task]
pub async fn arrow_app(_context: AppContext) {
    // 3D rendering API removed - app disabled
}
