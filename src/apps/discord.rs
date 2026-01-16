use crate::apps::components::{
    draw_background, draw_card, draw_list, draw_status_bar, AccentColor, BadgeConfig, BadgeTone,
    CardConfig, StatusBarData, StyleFonts, StyleMetrics, StylePalette,
};
use crate::libs::http;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use defmt::{info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};

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

    let status_label = if connected { "SYNCED" } else { "RETRYING" };
    let status_area = draw_status_bar(
        surface,
        &metrics,
        fonts,
        palette,
        StatusBarData {
            left: "Discord",
            battery_percent: 68,
            right: Some(status_label),
        },
    );

    let mut next_y = status_area.y2 + 1 + metrics.section_spacing;

    let message_lines: Vec<String> = messages
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            if msg.trim().is_empty() || msg.as_str() == "..." {
                format!("{:02}. ...", idx + 1)
            } else {
                format!("{:02}. {}", idx + 1, msg)
            }
        })
        .collect();
    let message_refs = message_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();

    let list_height =
        message_refs.len() as i32 * (fonts.line_height_body() + metrics.section_padding / 2);
    let message_card_height =
        metrics.section_padding * 2 + fonts.line_height_title() + 4 + list_height;

    let message_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Messages"),
            subtitle: Some("Latest channel updates"),
            badge: Some(BadgeConfig {
                text: if connected { "LIVE" } else { "OFF" },
                tone: if connected {
                    BadgeTone::Accent
                } else {
                    BadgeTone::Gray
                },
            }),
            accent: AccentColor::Yellow,
            height: message_card_height.max(metrics.button_height * 2),
        },
    );

    draw_list(surface, &message_card, fonts, palette, &message_refs);

    next_y = message_card.next_y(&metrics);

    if next_y >= metrics.height {
        return;
    }

    let delivered = messages.iter().filter(|m| m.as_str() != "...").count();
    let status_lines: Vec<String> = vec![
        if connected {
            "Status: Online".to_string()
        } else {
            "Status: Offline".to_string()
        },
        format!("Messages: {}/{}", delivered, messages.len()),
    ];
    let status_refs = status_lines
        .iter()
        .map(|line| line.as_str())
        .collect::<Vec<&str>>();
    let status_height = metrics.section_padding * 2
        + fonts.line_height_title()
        + 4
        + status_refs.len() as i32 * (fonts.line_height_body() + metrics.section_padding / 2);

    let status_card = draw_card(
        surface,
        &metrics,
        fonts,
        palette,
        next_y,
        CardConfig {
            title: Some("Summary"),
            subtitle: Some("Channel overview"),
            badge: None,
            accent: AccentColor::Gray,
            height: status_height.max(metrics.button_height * 2),
        },
    );

    draw_list(surface, &status_card, fonts, palette, &status_refs);
}
