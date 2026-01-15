use crate::apps::text::{ascii_text_width, draw_ascii_text, FONT_HEIGHT};
use crate::system::app::app_context::AppContext;
use crate::system::services::gyro_accel_srv::{
    latest_imu_snapshot, wait_for_imu_snapshot, ImuSnapshot,
};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::{format, string::String};
use embassy_executor::task;
use embassy_time::{with_timeout, Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{draw_rect, RectDsc};
use rust_gfx::types::{Area, Gradient, OPA_COVER};

#[task]
pub async fn imu_demo_app(ctx: AppContext) {
    let mut snapshot = latest_imu_snapshot().await;
    let mut last_update: Option<Instant> = snapshot.map(|_| Instant::now());

    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        if let Ok(new_snapshot) =
            with_timeout(Duration::from_millis(250), wait_for_imu_snapshot()).await
        {
            snapshot = Some(new_snapshot);
            last_update = Some(Instant::now());
        }

        let snapshot_value = snapshot.unwrap_or_else(empty_snapshot);
        let age_ms = last_update.map(|stamp| {
            let now = Instant::now();
            let elapsed = now - stamp;
            elapsed.as_millis() as u32
        });

        ctx.draw(|surface| draw_interface(surface, snapshot_value, age_ms))
            .await;
        ctx.request_redraw().await;
        Timer::after(Duration::from_millis(60)).await;
    }
}

fn draw_interface(surface: &mut DrawingSurface, snapshot: ImuSnapshot, age_ms: Option<u32>) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }

    let mut background = RectDsc::new();
    background.bg_color = Rgba8888::rgba(248, 249, 252, 255);
    background.bg_grad = Gradient::vertical(
        Rgba8888::rgba(255, 255, 255, 255),
        Rgba8888::rgba(232, 234, 240, 255),
    );
    background.bg_opa = OPA_COVER;
    let full_area = Area::new(0, 0, width - 1, height - 1);
    draw_rect(surface, &background, &full_area);

    let header_text = "IMU Monitor";
    let header_width = ascii_text_width(header_text);
    let header_x = (width - header_width) / 2;
    let header_y = 10;
    draw_ascii_text(
        surface,
        header_text,
        header_x,
        header_y,
        Rgba8888::rgba(60, 64, 80, 255),
    );

    let status_text = match age_ms {
        Some(ms) => format!("Δt {} ms", ms),
        None => String::from("Waiting for data"),
    };
    let status_width = ascii_text_width(status_text.as_str());
    let status_x = (width - status_width - 10).max(10);
    draw_ascii_text(
        surface,
        status_text.as_str(),
        status_x,
        header_y,
        Rgba8888::rgba(86, 92, 110, 255),
    );

    let (roll, pitch, yaw) = quaternion_to_euler_deg(snapshot.orientation);
    let orientation_lines = [
        format!("Roll : {:+05.1}°", roll),
        format!("Pitch: {:+05.1}°", pitch),
        format!("Yaw  : {:+05.1}°", yaw),
    ];

    let accel_lines = [
        format!("Ax: {:+.2} g", snapshot.accel.0),
        format!("Ay: {:+.2} g", snapshot.accel.1),
        format!("Az: {:+.2} g", snapshot.accel.2),
    ];

    let gyro_lines = [
        format!("Gx: {:+.1} dps", snapshot.gyro.0),
        format!("Gy: {:+.1} dps", snapshot.gyro.1),
        format!("Gz: {:+.1} dps", snapshot.gyro.2),
    ];

    let accel_mag = (snapshot.accel.0 * snapshot.accel.0
        + snapshot.accel.1 * snapshot.accel.1
        + snapshot.accel.2 * snapshot.accel.2)
        .sqrt();
    let summary_lines = [
        format!("|a| : {:.2} g", accel_mag),
        format!("Temp: {:.1} °C", snapshot.temperature_c),
    ];

    let mut y = header_y + FONT_HEIGHT + 12;
    y = draw_section(surface, width, height, y, "Orientation", &orientation_lines);
    y = draw_section(surface, width, height, y, "Acceleration", &accel_lines);
    y = draw_section(surface, width, height, y, "Gyroscope", &gyro_lines);
    let _ = draw_section(surface, width, height, y, "Status", &summary_lines);
}

fn draw_section(
    surface: &mut DrawingSurface,
    width: i32,
    height: i32,
    top: i32,
    title: &str,
    lines: &[String],
) -> i32 {
    if width <= 0 || height <= 0 || top >= height {
        return height;
    }

    let margin_x = 12;
    let card_width = width - margin_x * 2;
    if card_width <= 0 {
        return height;
    }

    let section_padding = 6;
    let accent_height = 3;
    let line_spacing = FONT_HEIGHT + 3;
    let content_height = (lines.len() as i32).saturating_mul(line_spacing);
    let card_height = section_padding * 2 + accent_height + FONT_HEIGHT + 4 + content_height;
    let bottom = (top + card_height - 1).min(height - 1);
    if bottom < top {
        return height;
    }

    let mut card = RectDsc::new();
    card.bg_color = Rgba8888::rgba(255, 255, 255, 235);
    card.bg_grad = Gradient::vertical(
        Rgba8888::rgba(250, 252, 255, 255),
        Rgba8888::rgba(230, 234, 242, 255),
    );
    card.bg_opa = OPA_COVER;
    card.radius = 8;
    card.border_width = 1;
    card.border_color = Rgba8888::rgba(200, 204, 216, 180);
    card.border_opa = OPA_COVER;
    let card_area = Area::new(margin_x, top, margin_x + card_width - 1, bottom);
    draw_rect(surface, &card, &card_area);

    let mut accent = RectDsc::new();
    accent.bg_color = Rgba8888::rgba(70, 190, 235, 180);
    accent.bg_opa = OPA_COVER;
    accent.radius = 1;
    let accent_area = Area::new(
        margin_x + 4,
        top + 4,
        margin_x + card_width - 5,
        (top + 4 + accent_height).min(bottom),
    );
    draw_rect(surface, &accent, &accent_area);

    let title_width = ascii_text_width(title);
    let title_x = margin_x + (card_width - title_width) / 2;
    let title_y = top + section_padding + accent_height + 2;
    draw_ascii_text(
        surface,
        title,
        title_x.max(margin_x + 4),
        title_y,
        Rgba8888::rgba(60, 64, 80, 255),
    );

    let mut line_y = title_y + FONT_HEIGHT + 4;
    let text_color = Rgba8888::rgba(86, 92, 110, 255);
    let content_x = margin_x + 12;
    for line in lines {
        if line_y > bottom {
            break;
        }
        draw_ascii_text(surface, line.as_str(), content_x, line_y, text_color);
        line_y += line_spacing;
    }

    (bottom + 10).min(height)
}

fn quaternion_to_euler_deg(q: Quaternion) -> (f32, f32, f32) {
    let sinr_cosp = 2.0 * (q.w * q.x + q.y * q.z);
    let cosr_cosp = 1.0 - 2.0 * (q.x * q.x + q.y * q.y);
    let roll = sinr_cosp.atan2(cosr_cosp);

    let sinp = 2.0 * (q.w * q.y - q.z * q.x);
    let pitch = if sinp.abs() >= 1.0 {
        core::f32::consts::FRAC_PI_2.copysign(sinp)
    } else {
        sinp.asin()
    };

    let siny_cosp = 2.0 * (q.w * q.z + q.x * q.y);
    let cosy_cosp = 1.0 - 2.0 * (q.y * q.y + q.z * q.z);
    let yaw = siny_cosp.atan2(cosy_cosp);

    let rad_to_deg = 57.29578_f32;
    (roll * rad_to_deg, pitch * rad_to_deg, yaw * rad_to_deg)
}

fn empty_snapshot() -> ImuSnapshot {
    ImuSnapshot {
        accel: Vec3(0.0, 0.0, 0.0),
        gyro: Vec3(0.0, 0.0, 0.0),
        temperature_c: 0.0,
        orientation: Quaternion::identity(),
    }
}
