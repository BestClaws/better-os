use crate::system::app::app_context::AppContext;
use crate::system::services::gyro_accel_srv::{
    latest_imu_snapshot, wait_for_imu_snapshot, ImuSnapshot,
};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::{
    borrow::Cow,
    format,
    string::{String, ToString},
};
use embassy_executor::task;
use embassy_time::{with_timeout, Duration, Instant, Timer};
use micromath::F32Ext;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{
    draw_rect,
    label::{draw_label, line_height_for_font, measure_text_with_font, FontId, LabelDsc},
    RectDsc,
};
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

    let header_font = font_for_role(width, height, FontRole::Header);
    let title_font = font_for_role(width, height, FontRole::Accent);
    let body_font = font_for_role(width, height, FontRole::Body);
    let header_height = font_height(header_font);
    let title_height = font_height(title_font);
    let body_height = font_height(body_font);
    let small_screen = width <= 120 || height <= 140;

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
    let header_width = text_width(header_text, header_font);
    let header_x = (width - header_width) / 2;
    let header_y = if small_screen { 2 } else { 8 };
    draw_text(
        surface,
        header_text,
        header_font,
        header_x,
        header_y,
        Rgba8888::rgba(60, 64, 80, 255),
    );

    let status_text = match age_ms {
        Some(ms) => format!("Δt:{}ms", ms.min(9_999)),
        None => String::from("Δt:----"),
    };
    let status_width = text_width(status_text.as_str(), body_font);
    let status_x = if small_screen {
        6
    } else {
        (width - status_width - 8).max(8)
    };
    let status_y = if small_screen {
        header_y + header_height + 2
    } else {
        header_y
    };
    draw_text(
        surface,
        status_text.as_str(),
        body_font,
        status_x,
        status_y,
        Rgba8888::rgba(86, 92, 110, 255),
    );

    let header_block_end = if small_screen {
        status_y + body_height
    } else {
        header_y + header_height
    };

    let (roll, pitch, yaw) = quaternion_to_euler_deg(snapshot.orientation);
    let orientation_lines = [
        format!("R:{:+05.1}°", roll),
        format!("P:{:+05.1}°", pitch),
        format!("Y:{:+05.1}°", yaw),
    ];

    let accel_lines = [
        format!("Ax:{:+.2}g", snapshot.accel.0),
        format!("Ay:{:+.2}g", snapshot.accel.1),
        format!("Az:{:+.2}g", snapshot.accel.2),
    ];

    let gyro_lines = [
        format!("Gx:{:+.1}dps", snapshot.gyro.0),
        format!("Gy:{:+.1}dps", snapshot.gyro.1),
        format!("Gz:{:+.1}dps", snapshot.gyro.2),
    ];

    let accel_mag = (snapshot.accel.0 * snapshot.accel.0
        + snapshot.accel.1 * snapshot.accel.1
        + snapshot.accel.2 * snapshot.accel.2)
        .sqrt();
    let summary_lines = [
        format!("|a|:{:.2}g", accel_mag),
        format!("T:{:.1}°C", snapshot.temperature_c),
    ];

    let mut y = header_block_end + if small_screen { 6 } else { 10 };
    y = draw_section(
        surface,
        width,
        height,
        y,
        "Orientation",
        &orientation_lines,
        title_font,
        body_font,
        small_screen,
    );
    y = draw_section(
        surface,
        width,
        height,
        y,
        "Acceleration",
        &accel_lines,
        title_font,
        body_font,
        small_screen,
    );
    y = draw_section(
        surface,
        width,
        height,
        y,
        "Gyroscope",
        &gyro_lines,
        title_font,
        body_font,
        small_screen,
    );
    let _ = draw_section(
        surface,
        width,
        height,
        y,
        "Status",
        &summary_lines,
        title_font,
        body_font,
        small_screen,
    );
}

fn draw_section(
    surface: &mut DrawingSurface,
    width: i32,
    height: i32,
    top: i32,
    title: &str,
    lines: &[String],
    title_font: FontId,
    body_font: FontId,
    small_screen: bool,
) -> i32 {
    if width <= 0 || height <= 0 || top >= height {
        return height;
    }

    let margin_x = if small_screen { 8 } else { 10 };
    let card_width = width - margin_x * 2;
    if card_width <= 0 {
        return height;
    }

    let section_padding = if small_screen { 5 } else { 6 };
    let accent_height = if small_screen { 2 } else { 3 };
    let title_height = font_height(title_font);
    let body_height = font_height(body_font);
    let line_spacing = body_height + if small_screen { 1 } else { 2 };
    let content_height = (lines.len() as i32).saturating_mul(line_spacing);
    let card_height =
        section_padding * 2 + accent_height + title_height + 4 + content_height;
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
    card.radius = 7;
    card.border_width = 1;
    card.border_color = Rgba8888::rgba(200, 204, 216, 160);
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

    let title_width = text_width(title, title_font);
    let title_x = margin_x + (card_width - title_width) / 2;
    let title_y = top + section_padding + accent_height + 2;
    draw_text(
        surface,
        title,
        title_font,
        title_x.max(margin_x + 4),
        title_y,
        Rgba8888::rgba(60, 64, 80, 255),
    );

    let mut line_y = title_y + title_height + 3;
    let text_color = Rgba8888::rgba(86, 92, 110, 255);
    let content_x = margin_x + if small_screen { 8 } else { 10 };
    let max_text_width = card_width - (content_x - margin_x) * 2;
    for line in lines {
        if line_y > bottom {
            break;
        }
        let (clamped, _) = clamp_text_to_width(line.as_str(), max_text_width, body_font);
        draw_text(surface, clamped.as_ref(), body_font, content_x, line_y, text_color);
        line_y += line_spacing;
    }

    (bottom + if small_screen { 6 } else { 8 }).min(height)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FontRole {
    Header,
    Body,
    Accent,
}

fn font_for_role(_width: i32, _height: i32, role: FontRole) -> FontId {
    match role {
        FontRole::Header => FontId::Montserrat12,
        FontRole::Body => FontId::Montserrat8,
        FontRole::Accent => FontId::Montserrat10,
    }
}

fn text_width(text: &str, font: FontId) -> i32 {
    if text.is_empty() {
        0
    } else {
        measure_text_with_font(text, 0, font)
    }
}

fn font_height(font: FontId) -> i32 {
    line_height_for_font(font)
}

fn draw_text(surface: &mut DrawingSurface, text: &str, font: FontId, x: i32, y: i32, color: Rgba8888) {
    let width = text_width(text, font);
    if width <= 0 {
        return;
    }
    let height = font_height(font);
    if height <= 0 {
        return;
    }
    let mut label = LabelDsc::new(String::from(text));
    label.font = font;
    label.color = color;
    let area = Area::new(x, y, x + width - 1, y + height - 1);
    draw_label(surface, &label, &area);
}

fn clamp_text_to_width<'a>(text: &'a str, max_width: i32, font: FontId) -> (Cow<'a, str>, i32) {
    if max_width <= 0 || text.is_empty() {
        return (Cow::Borrowed(""), 0);
    }

    let mut last_good_idx = 0;
    let mut last_width = 0;

    for (idx, ch) in text.char_indices() {
        let end = idx + ch.len_utf8();
        let candidate = &text[..end];
        let width = measure_text_with_font(candidate, 0, font);
        if width > max_width {
            break;
        }
        last_good_idx = end;
        last_width = width;
    }

    if last_good_idx == 0 {
        (Cow::Borrowed(""), 0)
    } else if last_good_idx == text.len() {
        (Cow::Borrowed(text), last_width)
    } else {
        (Cow::Owned(text[..last_good_idx].to_string()), last_width)
    }
}
