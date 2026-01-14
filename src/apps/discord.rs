use crate::libs::http;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};
use heapless::String as HString;
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::{
    draw_label,
    draw_rect,
    label::{LabelDsc, line_height, measure_text},
    RectDsc,
};
use rust_gfx::primitives::triangle::{draw_triangle, TriangleDsc};
use rust_gfx::types::{Area, Gradient, OPA_COVER, Point, RADIUS_CIRCLE};

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

fn update_messages_from_posts(messages: &mut [HString<MESSAGE_CAPACITY>; 3], body: &[u8]) -> bool {
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
                        messages[post_count].push_str(sanitized.as_str()).ok();
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
            messages[post_count].push_str("...").ok();
            changed = true;
        }
        post_count += 1;
    }

    changed
}

fn sanitize_line(line: &str) -> HString<MESSAGE_CAPACITY> {
    let mut sanitized = HString::<MESSAGE_CAPACITY>::new();
    for ch in line.chars() {
        if !ch.is_ascii() {
            continue;
        }
        if sanitized.len() >= MAX_DISPLAY_CHARS {
            break;
        }
        if sanitized.push(ch).is_err() {
            break;
        }
    }
    if sanitized.is_empty() {
        sanitized.push_str("...").ok();
    }
    sanitized
}

fn placeholder_message() -> HString<MESSAGE_CAPACITY> {
    let mut s = HString::<MESSAGE_CAPACITY>::new();
    let _ = s.push_str("...");
    s
}

fn draw_interface(
    surface: &mut DrawingSurface,
    messages: &[HString<MESSAGE_CAPACITY>; 3],
    connected: bool,
) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;

    if width <= 0 || height <= 0 {
        return;
    }

    // Background gradient
    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(32, 34, 40, 255);
    bg.bg_grad = Gradient::vertical(
        Rgba8888::rgba(38, 41, 48, 255),
        Rgba8888::rgba(18, 19, 24, 255),
    );
    bg.bg_opa = OPA_COVER;
    let bg_area = Area::new(0, 0, width - 1, height - 1);
    draw_rect(surface, &bg, &bg_area);

    // Header
    let header_height = (height / 6).max(18).min(30);
    let mut header = RectDsc::new();
    header.bg_color = Rgba8888::rgba(47, 49, 56, 255);
    header.bg_grad = Gradient::vertical(
        Rgba8888::rgba(63, 66, 74, 255),
        Rgba8888::rgba(35, 37, 42, 255),
    );
    header.bg_opa = OPA_COVER;
    let header_area = Area::new(0, 0, width - 1, header_height - 1);
    draw_rect(surface, &header, &header_area);

    let mut divider = RectDsc::new();
    divider.bg_color = Rgba8888::rgba(0, 0, 0, 90);
    divider.bg_opa = OPA_COVER;
    let divider_area = Area::new(0, header_height, width - 1, header_height);
    draw_rect(surface, &divider, &divider_area);

    // Header text and status
    let text_height = line_height();
    let mut title_label = LabelDsc::new("Discord".into());
    title_label.color = Rgba8888::rgba(242, 243, 245, 255);
    let title_width = measure_text(&title_label.text, title_label.letter_space);
    if title_width > 0 {
        let title_x = (width - title_width) / 2;
        let title_y = (header_height - text_height) / 2;
        let title_area = Area::new(
            title_x,
            title_y,
            title_x + title_width - 1,
            title_y + text_height - 1,
        );
        draw_label(surface, &title_label, &title_area);
    }

    let status_text = if connected { "ONLINE" } else { "OFFLINE" };
    let mut status_label = LabelDsc::new(status_text.into());
    status_label.color = if connected {
        Rgba8888::rgba(147, 197, 114, 255)
    } else {
        Rgba8888::rgba(200, 98, 98, 255)
    };
    let status_width = measure_text(&status_label.text, status_label.letter_space);

    // Status dot
    let dot_size = (width / 18).max(4).min(10);
    let dot_x = width - dot_size - 6;
    let dot_y = (header_height - dot_size) / 2;
    let mut dot = RectDsc::new();
    dot.bg_color = if connected {
        Rgba8888::rgba(78, 201, 138, 255)
    } else {
        Rgba8888::rgba(128, 132, 142, 255)
    };
    dot.bg_opa = OPA_COVER;
    dot.radius = RADIUS_CIRCLE;
    let dot_area = Area::new(dot_x, dot_y, dot_x + dot_size - 1, dot_y + dot_size - 1);
    draw_rect(surface, &dot, &dot_area);

    if status_width > 0 {
        let status_x = (dot_x - status_width - 6).max(6);
        let status_y = (header_height - text_height) / 2;
        let status_area = Area::new(
            status_x,
            status_y,
            status_x + status_width - 1,
            status_y + text_height - 1,
        );
        draw_label(surface, &status_label, &status_area);
    }

    // Message list layout
    let padding = (width / 18).max(6).min(18);
    let content_top = header_height + (height / 30).max(6).min(16);
    let available_height = (height - content_top).max(30);
    let message_spacing = (height / 50).max(4).min(10);
    let message_height = (available_height / 3 - message_spacing).max(18).min(40);
    let tail_width = (message_height / 2).max(5).min(12);
    let tail_half_height = (message_height / 3).max(5);
    let corner_radius = (width / 28).max(4).min(14);
    let text_padding = (message_height / 4).max(4).min(12);

    for (idx, message) in messages.iter().enumerate() {
        let align_right = idx % 2 == 1;
        let bubble_width = (width - padding * 2 - tail_width).max(message_height);
        if bubble_width <= 0 {
            continue;
        }

        let bubble_x = if align_right {
            width - padding - tail_width - bubble_width
        } else {
            padding + tail_width
        };
        let y = content_top + idx as i32 * (message_height + message_spacing);

        let bubble_top_color = if align_right {
            Rgba8888::rgba(88, 101, 242, 255)
        } else {
            Rgba8888::rgba(54, 57, 63, 255)
        };
        let bubble_bottom_color = if align_right {
            Rgba8888::rgba(71, 82, 196, 255)
        } else {
            Rgba8888::rgba(44, 47, 51, 255)
        };
        let text_color = if align_right {
            Rgba8888::rgba(238, 240, 255, 255)
        } else {
            Rgba8888::rgba(219, 222, 225, 255)
        };

        // Tail
        let base_x = if align_right {
            bubble_x + bubble_width - 1
        } else {
            bubble_x
        };
        let tip_x = if align_right {
            (base_x + tail_width).min(width - 1)
        } else {
            (bubble_x - tail_width).max(0)
        };
        let mid_y = y + message_height / 2;
        let top_y = (mid_y - tail_half_height).max(y);
        let bottom_y = (mid_y + tail_half_height).min(y + message_height - 1);

        let mut tail = TriangleDsc::new(
            Point::new(base_x, top_y),
            Point::new(base_x, bottom_y),
            Point::new(tip_x, mid_y),
        );
        tail.color = bubble_top_color;
        tail.opa = OPA_COVER;
        tail.grad = Gradient::vertical(bubble_top_color, bubble_bottom_color);
        draw_triangle(surface, &tail);

        // Shadow
        let shadow_dx = if align_right { -2 } else { 2 };
        let shadow_dy = 2;
        let mut sx1 = bubble_x + shadow_dx;
        let mut sx2 = sx1 + bubble_width - 1;
        let mut sy1 = y + shadow_dy;
        let mut sy2 = sy1 + message_height - 1;
        sx1 = sx1.max(0);
        sx2 = sx2.min(width - 1);
        sy1 = sy1.max(0);
        sy2 = sy2.min(height - 1);
        if sx1 <= sx2 && sy1 <= sy2 {
            let mut shadow = RectDsc::new();
            shadow.bg_color = Rgba8888::rgba(0, 0, 0, 90);
            shadow.bg_opa = OPA_COVER;
            shadow.radius = corner_radius;
            let shadow_area = Area::new(sx1, sy1, sx2, sy2);
            draw_rect(surface, &shadow, &shadow_area);
        }

        // Bubble body
        let mut bubble = RectDsc::new();
        bubble.bg_color = bubble_top_color;
        bubble.bg_grad = Gradient::vertical(bubble_top_color, bubble_bottom_color);
        bubble.bg_opa = OPA_COVER;
        bubble.radius = corner_radius;
        bubble.border_width = 1;
        bubble.border_color = Rgba8888::rgba(0, 0, 0, 120);
        bubble.border_opa = OPA_COVER;
        let bubble_area = Area::new(
            bubble_x,
            y,
            bubble_x + bubble_width - 1,
            y + message_height - 1,
        );
        draw_rect(surface, &bubble, &bubble_area);

        // Message text
        let text_width_msg = measure_text(message.as_str(), 0);
        if text_width_msg > 0 {
            let mut text_label = LabelDsc::new(message.as_str().into());
            text_label.color = text_color;
            let text_x = bubble_x + text_padding;
            let text_y = y + (message_height - text_height) / 2;
            let max_x = bubble_x + bubble_width - text_padding - 1;
            let text_x2 = (text_x + text_width_msg - 1).min(max_x);
            if text_x <= text_x2 {
                let text_area = Area::new(text_x, text_y, text_x2, text_y + text_height - 1);
                draw_label(surface, &text_label, &text_area);
            }
        }
    }
}
