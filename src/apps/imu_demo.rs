use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_status_bar, AccentColor, BadgeConfig, BadgeTone,
    CardConfig, CardFrame, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
};
use crate::system::app::app_context::AppContext;
use crate::system::services::tasks::gyro_accel_srv::{
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImuView {
    Orientation,
    Acceleration,
    Gyroscope,
}

impl ImuView {
    fn next(self) -> Self {
        match self {
            ImuView::Orientation => ImuView::Acceleration,
            ImuView::Acceleration => ImuView::Gyroscope,
            ImuView::Gyroscope => ImuView::Orientation,
        }
    }
}

#[task]
pub async fn imu_demo_app(ctx: AppContext) {
    let mut snapshot = latest_imu_snapshot().await;
    let mut last_update: Option<Instant> = snapshot.map(|_| Instant::now());
    let mut view = ImuView::Orientation;
    let mut view_started = Instant::now();

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

        let now = Instant::now();
        if now - view_started >= Duration::from_secs(2) {
            view = view.next();
            view_started = now;
        }

        let snapshot_value = snapshot.unwrap_or_else(empty_snapshot);
        let age_ms = last_update.map(|stamp| {
            let elapsed = now - stamp;
            elapsed.as_millis() as u32
        });

        let current_view = view;
        ctx.draw(|surface| draw_interface(surface, snapshot_value, age_ms, current_view))
            .await;
        ctx.request_redraw().await;
        Timer::after(Duration::from_millis(60)).await;
    }
}

fn draw_interface(
    surface: &mut DrawingSurface,
    snapshot: ImuSnapshot,
    age_ms: Option<u32>,
    view: ImuView,
) {
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: "IMU Monitor",
            battery_percent: 70,
            right: None,
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;
    let (card_title, card_subtitle, card_badge, card_accent, lines) = match view {
        ImuView::Orientation => {
            let (roll, pitch, yaw) = quaternion_to_euler_deg(snapshot.orientation);
            (
                "Orientation",
                "Euler",
                ("ATT", BadgeTone::Accent),
                AccentColor::Yellow,
                vec![
                    format!("R:{}°", compact_angle_deg(roll)),
                    format!("P:{}°", compact_angle_deg(pitch)),
                    format!("Y:{}°", compact_angle_deg(yaw)),
                ],
            )
        }
        ImuView::Acceleration => (
            "Acceleration",
            "Linear g",
            ("ACC", BadgeTone::Accent),
            AccentColor::Gray,
            vec![
                format!("X:{}g", compact_g(snapshot.accel.0)),
                format!("Y:{}g", compact_g(snapshot.accel.1)),
                format!("Z:{}g", compact_g(snapshot.accel.2)),
            ],
        ),
        ImuView::Gyroscope => (
            "Gyroscope",
            "Angular",
            ("GYR", BadgeTone::Gray),
            AccentColor::Dark,
            vec![
                format!("X:{}°/s", compact_rate(snapshot.gyro.0)),
                format!("Y:{}°/s", compact_rate(snapshot.gyro.1)),
                format!("Z:{}°/s", compact_rate(snapshot.gyro.2)),
            ],
        ),
    };

    let refs = lines
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
            title: Some(card_title),
            subtitle: Some(card_subtitle),
            badge: Some(BadgeConfig {
                text: card_badge.0,
                tone: card_badge.1,
            }),
            accent: card_accent,
            height: metrics.button_height * 2,
        },
        &refs,
    ) {
        Some(frame) => frame.next_y(&metrics),
        None => return,
    };

    let accel_mag = (snapshot.accel.0 * snapshot.accel.0
        + snapshot.accel.1 * snapshot.accel.1
        + snapshot.accel.2 * snapshot.accel.2)
        .sqrt();
    let summary_lines = vec![format!("Temp {:>5}", compact_temp(snapshot.temperature_c))];
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

    let has_subtitle = config.subtitle.is_some();
    let desired = card_height_for_lines(lines.len(), fonts, metrics, has_subtitle);
    let height = adjust_card_height(desired, metrics.button_height * 2, available);
    if height <= 0 {
        return None;
    }

    let header = card_header_offset(has_subtitle, fonts, metrics);
    let content_height = height - header - metrics.section_padding;
    if content_height <= 0 {
        return None;
    }

    let mut line_count = lines.len();
    while line_count > 0 && list_content_height(line_count, fonts) > content_height {
        line_count -= 1;
    }

    if line_count == 0 {
        return None;
    }

    config.height = height;
    let frame = draw_card(surface, metrics, fonts, palette, top, config);
    let display_lines = &lines[..line_count];
    draw_list(surface, &frame, fonts, palette, display_lines);
    Some(frame)
}

fn card_height_for_lines(
    line_count: usize,
    fonts: StyleFonts,
    metrics: &StyleMetrics,
    has_subtitle: bool,
) -> i32 {
    let header = card_header_offset(has_subtitle, fonts, metrics);
    let content = if line_count == 0 {
        fonts.line_height_body()
    } else {
        list_content_height(line_count, fonts)
    };
    header + content + metrics.section_padding
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

fn compact_angle_deg(value: f32) -> String {
    let rounded = value.round() as i16;
    if rounded >= 0 {
        format!("+{}", rounded)
    } else {
        rounded.to_string()
    }
}

fn compact_rate(value: f32) -> String {
    compact_angle_deg(value)
}

fn compact_temp(value: f32) -> String {
    compact_signed_float(value, 1, 1)
}

fn compact_g(value: f32) -> String {
    let magnitude = value.abs();
    let decimals = if magnitude < 0.95 {
        2
    } else if magnitude < 9.5 {
        1
    } else {
        0
    };
    compact_signed_float(value, decimals, 1)
}

fn compact_signed_float(value: f32, decimals: usize, fallback_decimals: usize) -> String {
    let mut s = if decimals == 0 {
        format!("{:+.0}", value)
    } else {
        format!("{:+.*}", decimals, value)
    };

    if decimals > 0 {
        trim_trailing_zeroes(&mut s);
    }

    if s == "+0" || s == "-0" {
        if fallback_decimals > 0 {
            let mut fallback = format!("{:+.*}", fallback_decimals, value);
            trim_trailing_zeroes(&mut fallback);
            fallback
        } else {
            "+0".to_string()
        }
    } else {
        s
    }
}

fn trim_trailing_zeroes(text: &mut String) {
    if let Some(dot) = text.find('.') {
        while text.len() > dot + 1 && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
}

fn card_header_offset(has_subtitle: bool, fonts: StyleFonts, metrics: &StyleMetrics) -> i32 {
    if has_subtitle {
        metrics.section_padding + fonts.line_height_title() + fonts.line_height_body() + 6
    } else {
        metrics.section_padding + fonts.line_height_title() + 4
    }
}

fn list_content_height(line_count: usize, fonts: StyleFonts) -> i32 {
    if line_count == 0 {
        return 0;
    }
    let count = line_count as i32;
    count * fonts.line_height_body() + (4 * count - 2)
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
