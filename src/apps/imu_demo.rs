use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_status_bar, AccentColor, BadgeConfig, BadgeTone,
    CardConfig, CardFrame, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
};
use crate::system::app::app_context::AppContext;
use crate::system::services::gyro_accel_srv::{
    latest_imu_snapshot, wait_for_imu_snapshot, ImuSnapshot,
};
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use embassy_executor::task;
use embassy_time::{with_timeout, Duration, Instant, Timer};
use micromath::F32Ext;

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
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let delta_label = age_ms
        .map(|ms| format!("Δt:{:>4}ms", ms.min(9_999)))
        .unwrap_or_else(|| "Δt:----".to_string());

    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: "IMU Monitor",
            battery_percent: 70,
            right: Some(delta_label.as_str()),
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;

    let (roll, pitch, yaw) = quaternion_to_euler_deg(snapshot.orientation);
    let orientation_lines = vec![
        format!("Roll   {:+06.2}°", roll),
        format!("Pitch  {:+06.2}°", pitch),
        format!("Yaw    {:+06.2}°", yaw),
    ];
    let orientation_refs = orientation_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();
    next_y = match draw_data_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Orientation"),
            subtitle: Some("Euler (deg)"),
            badge: Some(BadgeConfig {
                text: "ATT",
                tone: BadgeTone::Accent,
            }),
            accent: AccentColor::Yellow,
            height: metrics.button_height * 2,
        },
        &orientation_refs,
    ) {
        Some(frame) => frame.next_y(&metrics),
        None => return,
    };

    let accel_lines = vec![
        format!("X  {:+06.3} g", snapshot.accel.0),
        format!("Y  {:+06.3} g", snapshot.accel.1),
        format!("Z  {:+06.3} g", snapshot.accel.2),
    ];
    let accel_refs = accel_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();
    next_y = match draw_data_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Acceleration"),
            subtitle: Some("Linear (g)"),
            badge: Some(BadgeConfig {
                text: "ACC",
                tone: BadgeTone::Accent,
            }),
            accent: AccentColor::Gray,
            height: metrics.button_height * 2,
        },
        &accel_refs,
    ) {
        Some(frame) => frame.next_y(&metrics),
        None => return,
    };

    let gyro_lines = vec![
        format!("X  {:+07.2} dps", snapshot.gyro.0),
        format!("Y  {:+07.2} dps", snapshot.gyro.1),
        format!("Z  {:+07.2} dps", snapshot.gyro.2),
    ];
    let gyro_refs = gyro_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();
    next_y = match draw_data_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Gyroscope"),
            subtitle: Some("Angular (dps)"),
            badge: Some(BadgeConfig {
                text: "GYR",
                tone: BadgeTone::Gray,
            }),
            accent: AccentColor::Dark,
            height: metrics.button_height * 2,
        },
        &gyro_refs,
    ) {
        Some(frame) => frame.next_y(&metrics),
        None => return,
    };

    let accel_mag = (snapshot.accel.0 * snapshot.accel.0
        + snapshot.accel.1 * snapshot.accel.1
        + snapshot.accel.2 * snapshot.accel.2)
        .sqrt();
    let summary_age_line = age_ms
        .map(|ms| format!("Last update {:>4} ms", ms.min(9_999)))
        .unwrap_or_else(|| "Last update ---- ms".to_string());
    let summary_lines = vec![
        format!("‖a‖   {:>5.2} g", accel_mag),
        format!("Temp   {:>5.1} °C", snapshot.temperature_c),
        summary_age_line,
    ];
    let summary_refs = summary_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();
    let (badge_text, badge_tone) = freshness_badge(age_ms);
    let summary_accent = if accel_mag > 1.5 {
        AccentColor::Dark
    } else {
        AccentColor::Gray
    };

    let _ = draw_data_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Status"),
            subtitle: Some("Summary"),
            badge: Some(BadgeConfig {
                text: badge_text,
                tone: badge_tone,
            }),
            accent: summary_accent,
            height: metrics.button_height * 2,
        },
        &summary_refs,
    );
}

fn draw_data_card(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    top: i32,
    mut config: CardConfig,
    lines: &[&str],
) -> Option<CardFrame> {
    let available = metrics.height.saturating_sub(top);
    if available <= metrics.section_padding {
        return None;
    }

    let desired = card_height_for_lines(lines.len(), fonts, metrics);
    let height = adjust_card_height(desired, metrics.button_height * 2, available);
    if height <= 0 {
        return None;
    }

    config.height = height;
    let frame = draw_card(surface, metrics, fonts, palette, top, config);
    draw_list(surface, &frame, fonts, palette, lines);
    Some(frame)
}

fn card_height_for_lines(line_count: usize, fonts: StyleFonts, metrics: &StyleMetrics) -> i32 {
    if line_count == 0 {
        return metrics.section_padding * 2 + fonts.line_height_title();
    }

    let body = line_count as i32 * fonts.line_height_body();
    let dividers = line_count.saturating_sub(1) as i32 * (metrics.section_padding / 2);
    metrics.section_padding * 2
        + fonts.line_height_title()
        + metrics.section_padding / 2
        + body
        + dividers
}

fn adjust_card_height(desired: i32, min: i32, max: i32) -> i32 {
    if max <= 0 {
        return 0;
    }
    if max < min {
        max
    } else {
        desired.clamp(min, max)
    }
}

fn freshness_badge(age_ms: Option<u32>) -> (&'static str, BadgeTone) {
    match age_ms {
        Some(ms) if ms <= 500 => ("LIVE", BadgeTone::Accent),
        Some(ms) if ms <= 2_000 => ("LAG", BadgeTone::Gray),
        _ => ("STALE", BadgeTone::Danger),
    }
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
