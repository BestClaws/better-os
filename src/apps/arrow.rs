use crate::system::app::app_context::AppContext;
use crate::system::services::gyro_accel_srv::{latest_orientation, wait_for_orientation_update};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives as util_math;
use alloc::vec::Vec;
use defmt::{error, info};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use rust_gfx::color::Rgba8888;
use rust_gfx::three_d::{
    load_glb, render_scene, Camera, Mat4, Quaternion, RenderOptions, ShadingMode, Vec3,
};

const GLB_DATA: &[u8] = include_bytes!("../assets/arrow2.glb");

fn util_conjugate(q: util_math::Quaternion) -> util_math::Quaternion {
    util_math::Quaternion {
        w: q.w,
        x: -q.x,
        y: -q.y,
        z: -q.z,
    }
}

fn util_mul(a: util_math::Quaternion, b: util_math::Quaternion) -> util_math::Quaternion {
    util_math::Quaternion {
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
    }
}

fn util_normalize(q: util_math::Quaternion) -> util_math::Quaternion {
    let mag = q.magnitude();
    if mag > 1.0e-6 {
        util_math::Quaternion {
            w: q.w / mag,
            x: q.x / mag,
            y: q.y / mag,
            z: q.z / mag,
        }
    } else {
        util_math::Quaternion::identity()
    }
}

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

    let mut last_log = Instant::now();
    let mut frame_counter = 0u32;

    let mut orientation = latest_orientation()
        .await
        .unwrap_or_else(util_math::Quaternion::identity);
    let mut reference_orientation: Option<util_math::Quaternion> = None;

    let render_options = RenderOptions {
        mode: ShadingMode::Lit,
        overlay_wireframe: false,
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

        if let Ok(new_orientation) =
            with_timeout(Duration::from_millis(20), wait_for_orientation_update()).await
        {
            orientation = new_orientation;
        }

        orientation = util_normalize(orientation);
        if reference_orientation.is_none() {
            // Capture the initial pose so the arrow starts level when the device rests.
            reference_orientation = Some(orientation);
        }

        let reference = reference_orientation.unwrap();
        let relative = util_mul(util_conjugate(reference), orientation);
        let adjusted = util_conjugate(relative);
        let adjusted = util_normalize(adjusted);

        let gfx_orientation = Quaternion::new(adjusted.w, adjusted.x, adjusted.y, adjusted.z);
        let rotation = Mat4::from_quaternion(gfx_orientation);
        for (node, base) in scene.nodes.iter_mut().zip(base_transforms.iter()) {
            let mut updated = rotation.mul_mat4(base);
            updated.m[0][3] = base.m[0][3];
            updated.m[1][3] = base.m[1][3];
            updated.m[2][3] = base.m[2][3];
            node.transform = updated;
        }

        context
            .draw(|surface: &mut DrawingSurface| {
                let width = surface.width() as f32;
                let height = surface.height() as f32;
                let aspect = if height > 0.0 { width / height } else { 1.0 };

                let camera = Camera::look_at_perspective(
                    Vec3::new(0.0, 0.0, 3.0),
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
        Timer::after(Duration::from_millis(5)).await;
    }
}
