use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_progress_bar, draw_status_bar, AccentColor,
    BadgeConfig, BadgeTone, CardConfig, ProgressBarConfig, StatusBarData, StyleFonts, StyleMetrics,
    StylePalette,
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::{format, string::String, vec, vec::Vec};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};
use esp_alloc::{HeapStats, HEAP};

#[task]
pub async fn heap_monitor_app(ctx: AppContext) {
    let mut ticker = Ticker::every(Duration::from_secs(1));

    loop {
        if !ctx.is_focused().await {
            ticker.next().await;
            continue;
        }

        let stats = HEAP.stats();
        ctx.draw(|surface| draw_interface(surface, &stats)).await;
        ticker.next().await;
    }
}

fn draw_interface(surface: &mut DrawingSurface, stats: &HeapStats) {
    let metrics = StyleMetrics::from_surface(surface);
    let palette = StylePalette::arknights();
    let fonts = StyleFonts::for_surface(metrics.width, metrics.height);

    draw_background(surface, &metrics, palette);

    let total = stats.size;
    let used = stats.current_usage;
    let free = total.saturating_sub(used);
    let percent = if total == 0 {
        0
    } else {
        ((used as u128 * 100) / total as u128) as u32
    };

    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: "Heap Monitor",
            battery_percent: 68,
            right: Some("ACTIVE"),
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;

    let info_lines: Vec<String> = vec![
        format!("Tot {}", format_size(total)),
        format!("Use {}", format_size(used)),
        format!("Fre {}", format_size(free)),
        format!("Load {:>3}%", percent.min(999)),
    ];
    let info_refs = info_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();

    let base_height = list_card_height(info_refs.len(), fonts, &metrics);
    let bar_height = metrics.section_padding + 2;
    let label_height = fonts.line_height_small();
    let gauge_reserve = bar_height + label_height + 2;
    let card_height = base_height + gauge_reserve;
    let heap_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Allocator"),
            subtitle: Some("Current usage"),
            badge: Some(BadgeConfig {
                text: if percent >= 85 { "HIGH" } else { "STABLE" },
                tone: if percent >= 85 {
                    BadgeTone::Danger
                } else {
                    BadgeTone::Accent
                },
            }),
            accent: if percent >= 85 {
                AccentColor::Dark
            } else {
                AccentColor::Yellow
            },
            height: card_height,
        },
    );

    let mut list_frame = heap_card;
    let content_height = list_frame
        .content_area
        .y2
        .saturating_sub(list_frame.content_area.y1);
    let reserve = gauge_reserve.min(content_height).max(0);
    let new_bottom = list_frame.content_area.y2.saturating_sub(reserve);
    list_frame.content_area.y2 = if new_bottom < list_frame.content_area.y1 {
        list_frame.content_area.y1
    } else {
        new_bottom
    };

    draw_list(surface, &list_frame, fonts, palette, &info_refs);

    draw_progress_bar(
        surface,
        &heap_card,
        &metrics,
        fonts,
        palette,
        ProgressBarConfig {
            percent: percent.min(100) as u8,
            label: "",
        },
    );
}

fn list_card_height(line_count: usize, fonts: StyleFonts, metrics: &StyleMetrics) -> i32 {
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

fn format_size(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    if bytes >= MB {
        let whole = bytes / MB;
        let tenths = ((bytes % MB) * 10) / MB;
        if tenths > 0 {
            format!("{whole}.{tenths}MB")
        } else {
            format!("{whole}MB")
        }
    } else {
        let kilobytes = bytes / KB;
        let tenths = ((bytes % KB) * 10) / KB;
        if kilobytes >= 100 || tenths == 0 {
            format!("{kilobytes}KB")
        } else {
            format!("{kilobytes}.{tenths}KB")
        }
    }
}
