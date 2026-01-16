use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_status_bar, AccentColor, BadgeConfig, BadgeTone,
    CardConfig, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
};
use crate::system::app::app_context::AppContext;
use crate::system::services::gyro_accel_srv::{latest_orientation, wait_for_orientation_update};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives as util_math;
use alloc::{format, string::String, vec, vec::Vec};
use defmt::{error, info};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::rectangle::{draw_rect, RectDsc};
use rust_gfx::three_d::{
    load_glb, render_scene, Camera, Mat4, Quaternion, RenderOptions, ShadingMode, Vec3,
};
use rust_gfx::types::{Area, Gradient, OPA_COVER};

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

    let mut last_log = Instant::now();
    let mut frame_counter = 0u32;
    let mut last_fps = 0.0f32;

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

        orientation = orientation.normalize();
        if reference_orientation.is_none() {
            // Capture the initial pose so the arrow starts level when the device rests.
            reference_orientation = Some(orientation);
        }

        let reference = reference_orientation.unwrap();
        let relative = reference.conjugate().mul(&orientation);
        let adjusted = relative.conjugate().normalize();

        let gfx_orientation = Quaternion::new(adjusted.w, adjusted.x, adjusted.y, adjusted.z);
        let rotation = Mat4::from_quaternion(gfx_orientation);
        for (node, base) in scene.nodes.iter_mut().zip(base_transforms.iter()) {
            let mut updated = rotation.mul_mat4(base);
            updated.m[0][3] = base.m[0][3];
            updated.m[1][3] = base.m[1][3];
            updated.m[2][3] = base.m[2][3];
            node.transform = updated;
        }

        let (roll_deg, pitch_deg, yaw_deg) = quaternion_to_euler_deg(adjusted);
        let fps_snapshot = last_fps;

        context
            .draw(|surface: &mut DrawingSurface| {
                let metrics = StyleMetrics::from_surface(surface);
                let palette = StylePalette::arknights();
                let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

                draw_background(surface, &metrics, palette);

                let status_data = StatusBarData {
                    left: "Arrow",
                    battery_percent: 72,
                    right: Some("ACTIVE"),
                };

                let status_frame = draw_status_bar(surface, &metrics, fonts, palette, status_data);

                let mut next_y = status_frame.y2 + 1 + metrics.section_spacing;

                let available_height = metrics.height.saturating_sub(next_y);
                let summary_reserve = metrics.button_height * 2 + metrics.section_spacing;
                let viewport_height = available_height
                    .saturating_sub(summary_reserve)
                    .max(metrics.button_height * 3);
                let viewport_bottom = (next_y + viewport_height - 1).min(metrics.height - 1);
                let viewport_area = Area::new(
                    metrics.content_x(),
                    next_y,
                    metrics.content_x() + metrics.content_width() - 1,
                    viewport_bottom,
                );

                let mut viewport_bg = RectDsc::new();
                viewport_bg.bg_color = palette.container;
                viewport_bg.bg_grad = Gradient::vertical(palette.container, palette.container_alt);
                viewport_bg.bg_opa = OPA_COVER;
                viewport_bg.radius = metrics.section_radius;
                draw_rect(surface, &viewport_bg, &viewport_area);

                let mut accent = RectDsc::new();
                accent.bg_color = palette.accent_yellow;
                accent.bg_opa = OPA_COVER;
                accent.radius = metrics.section_radius.max(3);
                let accent_width = 4;
                let accent_area = Area::new(
                    viewport_area.x1,
                    viewport_area.y1,
                    (viewport_area.x1 + accent_width).min(viewport_area.x2),
                    viewport_area.y2,
                );
                draw_rect(surface, &accent, &accent_area);

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

                render_scene(surface, &scene, &camera, &render_options);

                draw_rect(surface, &accent, &accent_area);
                draw_status_bar(surface, &metrics, fonts, palette, status_data);

                next_y = viewport_area.y2 + 1 + metrics.section_spacing;

                let available = metrics.height.saturating_sub(next_y);
                if available > metrics.section_padding {
                    let orientation_lines: Vec<String> = vec![
                        format!("FPS  : {:4.1}", fps_snapshot),
                        format!("Roll : {:+05.1}°", roll_deg),
                        format!("Pitch: {:+05.1}°", pitch_deg),
                        format!("Yaw  : {:+05.1}°", yaw_deg),
                    ];
                    let orientation_refs = orientation_lines
                        .iter()
                        .map(|line| line.as_str())
                        .collect::<Vec<&str>>();

                    let dividers = orientation_refs.len().saturating_sub(1) as i32;
                    let content_height = orientation_refs.len() as i32 * fonts.line_height_body()
                        + dividers * (metrics.section_padding / 2);

                    let mut card_height = metrics.section_padding * 2
                        + fonts.line_height_title()
                        + metrics.section_padding / 2
                        + content_height;
                    let min_height = metrics.button_height * 2;
                    let max_height = available;
                    if max_height > 0 {
                        let card_height = if max_height < min_height {
                            max_height
                        } else {
                            card_height.clamp(min_height, max_height)
                        };

                        if card_height > 0 {
                            let summary_card = draw_card(
                                surface,
                                &metrics,
                                fonts,
                                palette,
                                next_y,
                                CardConfig {
                                    title: Some("Orientation"),
                                    subtitle: Some("Euler (deg)"),
                                    badge: Some(BadgeConfig {
                                        text: "LIVE",
                                        tone: BadgeTone::Accent,
                                    }),
                                    accent: AccentColor::Yellow,
                                    height: card_height,
                                },
                            );

                            draw_list(surface, &summary_card, fonts, palette, &orientation_refs);
                        }
                    }
                }
            })
            .await;

        frame_counter += 1;
        let now = Instant::now();
        if (now - last_log).as_millis() >= 1000 {
            let fps = frame_counter as f32 * 1000.0 / (now - last_log).as_millis() as f32;
            info!("arrow fps: {=f32}", fps);
            last_fps = fps;
            frame_counter = 0;
            last_log = now;
        }

        context.request_redraw().await;
        Timer::after(Duration::from_millis(5)).await;
    }
}

fn quaternion_to_euler_deg(q: util_math::Quaternion) -> (f32, f32, f32) {
    let ysqr = q.y * q.y;

    let t0 = 2.0 * (q.w * q.x + q.y * q.z);
    let t1 = 1.0 - 2.0 * (q.x * q.x + ysqr);
    let roll = t0.atan2(t1);

    let mut t2 = 2.0 * (q.w * q.y - q.z * q.x);
    t2 = t2.clamp(-1.0, 1.0);
    let pitch = t2.asin();

    let t3 = 2.0 * (q.w * q.z + q.x * q.y);
    let t4 = 1.0 - 2.0 * (ysqr + q.z * q.z);
    let yaw = t3.atan2(t4);

    (roll.to_degrees(), pitch.to_degrees(), yaw.to_degrees())
}
