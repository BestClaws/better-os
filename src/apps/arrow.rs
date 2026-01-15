use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::vec::Vec;
use defmt::{error, info};
use embassy_time::{Duration, Instant, Timer};
use rust_gfx::color::Rgba8888;
use rust_gfx::three_d::{
    load_glb, render_scene, Camera, Mat4, Quaternion, RenderOptions, ShadingMode, Vec3,
};

const GLB_DATA: &[u8] = include_bytes!("../assets/arrow2.glb");

#[embassy_executor::task]
pub async fn arrow_app(context: AppContext) {
    let mut scene = match load_glb(GLB_DATA) {
        Ok(scene) => scene,
        Err(err) => {
            error!("arrow glb load failed: {:?}", defmt::Debug2Format(&err));
            return;
        }
    };

    if scene.meshes.is_empty() || scene.nodes.is_empty() {
        error!("arrow scene missing meshes");
        return;
    }

    let base_transforms: Vec<Mat4> = scene.nodes.iter().map(|node| node.transform).collect();

    let mut angle = 0.0f32;
    let mut last_log = Instant::now();
    let mut frame_counter = 0u32;

    let render_options = RenderOptions {
        mode: ShadingMode::Lit,
        overlay_wireframe: true,
        wireframe_color: Rgba8888::rgb(90, 200, 255),
        light_direction: Vec3::new(0.4, -0.6, -1.0),
        ambient_intensity: 0.25,
        diffuse_intensity: 0.65,
        specular_intensity: 0.2,
        shininess: 24.0,
        enable_backface_culling: true,
    };

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        angle += 0.04;
        for (node, base) in scene.nodes.iter_mut().zip(base_transforms.iter()) {
            let rotation = Mat4::from_quaternion(Quaternion::from_axis_angle(
                Vec3::new(0.0, 1.0, 0.0),
                angle,
            ));
            node.transform = rotation.mul_mat4(base);
        }

        context
            .draw(|surface: &mut DrawingSurface| {
                let width = surface.width() as f32;
                let height = surface.height() as f32;
                let aspect = if height > 0.0 { width / height } else { 1.0 };

                let camera = Camera::look_at_perspective(
                    Vec3::new(0.0, 0.0, 4.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    45.0f32.to_radians(),
                    aspect,
                    0.1,
                    20.0,
                );

                surface.clear(Rgba8888::BLACK);
                render_scene(surface, &scene, &camera, &render_options);
            })
            .await;

        frame_counter += 1;
        let now = Instant::now();
        if (now - last_log).as_millis() >= 1000 {
            let fps = frame_counter as f32 * 1000.0 / (now - last_log).as_millis() as f32;
            info!("arrow fps: {=f32}", fps);
            frame_counter = 0;
            last_log = now;
        }

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}
