//! Audio control app – manually starts the square-wave loop and reports each stage.

use alloc::{format, string::String};

use crate::apps::components::{
    draw_background, draw_card, draw_text, AccentColor, CardConfig, StyleFonts, StyleMetrics,
    StylePalette,
};
use crate::system::app::app_context::AppContext;
use crate::system::services::audio_srv::AudioService;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{info, warn};
use embassy_time::{Duration, Timer};
use rust_gfx::types::Area;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AudioStage {
    Idle,
    WaitingForDriver,
    TriggeringBeep,
    BeepPlayed,
    Failed,
}

impl AudioStage {
    fn index(self) -> usize {
        match self {
            AudioStage::Idle => 0,
            AudioStage::WaitingForDriver => 1,
            AudioStage::TriggeringBeep => 2,
            AudioStage::BeepPlayed | AudioStage::Failed => 3,
        }
    }

    fn title(self) -> &'static str {
        match self {
            AudioStage::Idle => "Idle",
            AudioStage::WaitingForDriver => "Waiting for driver",
            AudioStage::TriggeringBeep => "Triggering beep",
            AudioStage::BeepPlayed => "Beep played",
            AudioStage::Failed => "Failed",
        }
    }
}

const PIPELINE_STAGES: [&str; 4] = [
    "Idle",
    "Driver registered",
    "Trigger sent",
    "Beep completed",
];

#[embassy_executor::task]
pub async fn audio_app(ctx: AppContext) {
    info!("Audio app starting");
    let mut stage = AudioStage::Idle;
    let mut detail = String::new();
    let mut beep_played = false;

    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(200)).await;
            continue;
        }

        if beep_played {
            render_status(&ctx, stage, Some(detail.as_str())).await;
            Timer::after(Duration::from_secs(1)).await;
            continue;
        }

        // Stage: waiting for driver registration
        stage = AudioStage::WaitingForDriver;
        detail.clear();
        render_status(&ctx, stage, None).await;

        stage = AudioStage::TriggeringBeep;
        render_status(&ctx, stage, None).await;

        match AudioService::play_minute_beep().await {
            Ok(()) => {
                stage = AudioStage::BeepPlayed;
                detail = String::from("Minute chime triggered");
                beep_played = true;
                info!("Minute chime played successfully");
            }
            Err(err) => {
                stage = AudioStage::Failed;
                detail = format!("Beep failed: {:?}", err);
                warn!("Minute chime failed: {:?}", err);
            }
        }
        render_status(&ctx, stage, Some(detail.as_str())).await;
        Timer::after(Duration::from_millis(500)).await;
    }
}

async fn render_status(ctx: &AppContext, stage: AudioStage, detail: Option<&str>) {
    ctx.draw(|surface: &mut DrawingSurface| {
        draw_view(surface, stage, detail);
    })
    .await;
}

fn draw_view(surface: &mut DrawingSurface, stage: AudioStage, detail: Option<&str>) {
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let card_height = (metrics.button_height * 3).max(metrics.section_padding * 6);
    let frame = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        metrics.outer_padding,
        CardConfig {
            title: Some("Audio Pipeline"),
            subtitle: Some(stage.title()),
            badge: None,
            accent: if stage == AudioStage::Failed {
                AccentColor::Gray
            } else {
                AccentColor::Yellow
            },
            height: card_height,
        },
    );

    draw_stage_list(
        surface,
        &frame.content_area,
        &metrics,
        fonts,
        palette,
        stage,
    );

    if let Some(text) = detail {
        let detail_y = frame.content_area.y2 + metrics.section_padding;
        draw_detail(surface, &metrics, fonts, palette, detail_y, text);
    }
}

fn draw_stage_list(
    surface: &mut DrawingSurface,
    area: &Area,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    stage: AudioStage,
) {
    let mut y = area.y1;
    let text_x = area.x1 + metrics.section_padding;
    let active_idx = stage.index();

    for (idx, label) in PIPELINE_STAGES.iter().enumerate() {
        let indicator = if idx == active_idx { '>' } else { '-' };
        let text = format!("{} {}", indicator, label);
        let color = if idx <= active_idx {
            palette.text_primary
        } else {
            palette.text_muted
        };
        draw_text(surface, text.as_str(), fonts.body, text_x, y, color);
        y += fonts.line_height_body() + 4;
    }
}

fn draw_detail(
    surface: &mut DrawingSurface,
    metrics: &StyleMetrics,
    fonts: StyleFonts,
    palette: StylePalette,
    top: i32,
    text: &str,
) {
    let height = fonts.line_height_small() + metrics.section_padding * 2;
    let detail_frame = draw_card(
        surface,
        metrics,
        fonts,
        palette,
        top,
        CardConfig {
            title: Some("Status"),
            subtitle: None,
            badge: None,
            accent: AccentColor::Dark,
            height,
        },
    );
    let content_y = detail_frame.content_area.y1;
    draw_text(
        surface,
        text,
        fonts.small,
        detail_frame.content_area.x1,
        content_y,
        palette.text_secondary,
    );
}
