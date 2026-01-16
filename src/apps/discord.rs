use crate::apps::components::{
    draw_background, draw_badge, draw_status_bar, draw_text, BadgeConfig, BadgeTone, StatusBarData,
    StyleFonts, StyleMetrics, StylePalette,
};
use crate::libs::http;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::string::{String, ToString};
use defmt::{info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};
use rust_gfx::primitives::label::measure_text_with_font;
use rust_gfx::primitives::rectangle::{draw_rect, RectDsc};
use rust_gfx::types::{Area, OPA_COVER};

const MESSAGE_CAPACITY: usize = 64;
const POLL_INTERVAL_MS: u64 = 5_000; // Poll every 5 seconds
const API_URL: &str = "https://jsonplaceholder.typicode.com/posts?_limit=3"; // Only fetch 3 posts
const MAX_DISPLAY_CHARS: usize = 36;

#[task]
pub async fn discord_app(ctx: AppContext) {
    info!("Starting Discord app");

    let client = http::Client::new();
    let mut messages = [
        placeholder_message(),
        placeholder_message(),
        placeholder_message(),
    ];
    let mut connected = false;
    let mut ticker = Ticker::every(Duration::from_millis(POLL_INTERVAL_MS));
    let mut needs_redraw = true;

    info!("Discord: Entering main loop");
    loop {
        if needs_redraw && ctx.is_focused().await {
            info!("Discord: Drawing interface");
            ctx.draw(|surface| draw_interface(surface, &messages, connected))
                .await;
            needs_redraw = false;
        }

        ticker.next().await;

        match client.get_secure(API_URL).send().await {
            Ok(response) => {
                if !connected {
                    connected = true;
                    needs_redraw = true;
                }

                if response.is_success() {
                    if let Ok(text) = response.text() {
                        if update_messages_from_posts(&mut messages, text.as_bytes()) {
                            needs_redraw = true;
                        }
                    }
                }
            }
            Err(err) => {
                warn!("HTTP request failed: {:?}", err);
                if connected {
                    connected = false;
                    needs_redraw = true;
                }
                embassy_time::Timer::after(Duration::from_secs(10)).await;
            }
        }
    }
}

fn update_messages_from_posts(messages: &mut [String; 3], body: &[u8]) -> bool {
    let text = core::str::from_utf8(body).unwrap_or("");
    info!("Discord: Parsing {} bytes of JSON text", text.len());
    let mut changed = false;

    // Simple JSON parsing - look for "title": "..." patterns
    let mut post_count = 0;

    for line in text.lines() {
        if post_count >= 3 {
            break;
        }

        // Look for "title": "some title here"
        if let Some(title_start) = line.find("\"title\":") {
            if let Some(first_quote) = line[title_start + 8..].find('"') {
                let start = title_start + 8 + first_quote + 1;
                if let Some(end_quote) = line[start..].find('"') {
                    let title = &line[start..start + end_quote];
                    let sanitized = sanitize_line(title);

                    if messages[post_count].as_str() != sanitized.as_str() {
                        messages[post_count].clear();
                        messages[post_count].push_str(sanitized.as_str());
                        changed = true;
                    }

                    post_count += 1;
                }
            }
        }
    }

    // If we found fewer than 3 posts, mark as changed if slots were not empty
    while post_count < 3 {
        if !messages[post_count].is_empty() && messages[post_count].as_str() != "..." {
            messages[post_count].clear();
            messages[post_count].push_str("...");
            changed = true;
        }
        post_count += 1;
    }

    changed
}

fn sanitize_line(line: &str) -> String {
    let mut sanitized = String::with_capacity(MESSAGE_CAPACITY);
    for ch in line.chars() {
        if !ch.is_ascii() {
            continue;
        }
        if sanitized.len() >= MAX_DISPLAY_CHARS {
            break;
        }
        sanitized.push(ch);
    }
    if sanitized.is_empty() {
        sanitized.push_str("...");
    }
    sanitized
}

fn placeholder_message() -> String {
    let mut s = String::with_capacity(MESSAGE_CAPACITY);
    s.push_str("...");
    s
}

fn draw_interface(surface: &mut DrawingSurface, messages: &[String; 3], connected: bool) {
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
            left: "Discord",
            battery_percent: 68,
            right: None,
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;

    let status_text = if connected { "ONLINE" } else { "OFFLINE" };
    let chip_tone = if connected {
        BadgeTone::Accent
    } else {
        BadgeTone::Gray
    };
    let chip_baseline = next_y + fonts.line_height_small() + metrics.section_padding;
    draw_badge(
        surface,
        fonts,
        palette,
        BadgeConfig {
            text: status_text,
            tone: chip_tone,
        },
        metrics.content_x() + metrics.content_width(),
        chip_baseline,
    );
    next_y = chip_baseline + metrics.section_spacing;

    draw_message_bubbles(surface, messages, fonts, &metrics, palette, next_y);
}

fn draw_message_bubbles(
    surface: &mut DrawingSurface,
    messages: &[String; 3],
    fonts: StyleFonts,
    metrics: &StyleMetrics,
    palette: StylePalette,
    top: i32,
) {
    let padding_x = metrics.section_padding;
    let padding_y = (metrics.section_padding / 2).max(3);
    let gap = (metrics.section_spacing / 2).max(3);
    let mut y = top;
    let left = metrics.content_x();
    let width = metrics.content_width();
    let bottom_limit = metrics.height - metrics.outer_padding;

    for msg in messages.iter() {
        let text = if msg.trim().is_empty() || msg.as_str() == "..." {
            "..."
        } else {
            msg.as_str()
        };

        let text_width = measure_text_with_font(text, 0, fonts.body).min(width - padding_x * 2);
        let bubble_width = (text_width + padding_x * 2).min(width);
        let bubble_height = fonts.line_height_body() + padding_y * 2;

        if y + bubble_height > bottom_limit {
            break;
        }

        let bubble_left = left + (width - bubble_width) / 2;
        let bubble_area = Area::new(
            bubble_left,
            y,
            bubble_left + bubble_width - 1,
            y + bubble_height,
        );

        let mut bubble = RectDsc::new();
        bubble.bg_color = palette.container;
        bubble.bg_opa = OPA_COVER;
        bubble.radius = metrics.section_radius.max(4);
        bubble.border_width = 1;
        bubble.border_color = palette.outline;
        bubble.border_opa = 160;
        draw_rect(surface, &bubble, &bubble_area);

        let text_x = bubble_area.x1 + padding_x;
        let text_y = bubble_area.y1 + padding_y;
        draw_text(
            surface,
            text,
            fonts.body,
            text_x,
            text_y,
            palette.text_primary,
        );

        y = bubble_area.y2 + 1 + gap;
    }
}
