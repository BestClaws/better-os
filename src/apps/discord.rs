use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::{RoundedRect, Shape, SurfaceDrawTarget};
use crate::libs::http;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{info, warn};
use embassy_executor::task;
use embassy_time::{Duration, Ticker};
use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_5X8};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text as EgText;
use heapless::String;

const MESSAGE_CAPACITY: usize = 64;
const POLL_INTERVAL_MS: u64 = 5_000;  // Poll every 5 seconds
const API_URL: &str = "https://jsonplaceholder.typicode.com/posts?_limit=3";  // Only fetch 3 posts
const MAX_DISPLAY_CHARS: usize = 36;

#[task]
pub async fn discord_app(ctx: AppContext) {
    info!("Starting Posts viewer app");
    
    let client = http::Client::new();
    let mut messages = [
        placeholder_message(),
        placeholder_message(),
        placeholder_message(),
    ];
    let mut connected = false;
    let mut ticker = Ticker::every(Duration::from_millis(POLL_INTERVAL_MS));
    let mut needs_redraw = true;

    info!("Posts: Entering main loop");
    loop {
        if needs_redraw && ctx.is_focused().await {
            info!("Posts: Drawing interface");
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

fn update_messages_from_posts(messages: &mut [String<MESSAGE_CAPACITY>; 3], body: &[u8]) -> bool {
    let text = core::str::from_utf8(body).unwrap_or("");
    info!("Posts: Parsing {} bytes of JSON text", text.len());
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

fn sanitize_line(line: &str) -> String<MESSAGE_CAPACITY> {
    let mut sanitized = String::<MESSAGE_CAPACITY>::new();
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

fn placeholder_message() -> String<MESSAGE_CAPACITY> {
    let mut s = String::<MESSAGE_CAPACITY>::new();
    let _ = s.push_str("...");
    s
}

fn draw_interface(
    surface: &mut DrawingSurface,
    messages: &[String<MESSAGE_CAPACITY>; 3],
    connected: bool,
) {
    let width = surface.width() as i32;
    let height = surface.height() as i32;
    
    // Clean gradient background - use RoundedRect with 0 radius instead of fill_rect
    RoundedRect::new(0, 0, width, height, 0, 0, 0, 0)
        .fill_solid(Rgba8888::rgba(24, 25, 28, 255))
        .draw(surface);

    // Simple, clean header - use RoundedRect with 0 radius instead of fill_rect
    let header_height = 22;
    RoundedRect::new(0, 0, width, header_height, 0, 0, 0, 0)
        .fill_solid(Rgba8888::rgba(32, 34, 37, 255))
        .draw(surface);
    RoundedRect::new(0, header_height, width, 1, 0, 0, 0, 0)
        .fill_solid(Rgba8888::rgba(0, 0, 0, 60))
        .draw(surface);

    let title_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(242, 243, 245));
    let message_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(219, 222, 225));

    // Centered title
    let title_text = "Posts";
    let title_width = title_text.len() as i32 * FONT_5X8.character_size.width as i32;
    let title_x = (width - title_width) / 2;
    let title_y = 7;

    // Small status dot
    let dot_size = 6;
    let dot_x = width - 10;
    let dot_y = (header_height - dot_size) / 2;
    
    RoundedRect::new(dot_x, dot_y, dot_size, dot_size, 3, 3, 3, 3)
        .fill_solid(if connected {
            Rgba8888::rgba(67, 181, 129, 255)
        } else {
            Rgba8888::rgba(128, 132, 142, 255)
        })
        .draw(surface);

    // Clean message list
    let content_top = header_height + 6;
    let padding = 4;
    let message_width = width - (padding * 2);
    let message_height = 22;
    let message_spacing = 2;

    for (idx, message) in messages.iter().enumerate() {
        let x = padding;
        let y = content_top + idx as i32 * (message_height + message_spacing);

        // Clean message bubble
        RoundedRect::new(x, y, message_width, message_height, 8, 8, 8, 8)
            .fill_solid(Rgba8888::rgba(43, 45, 49, 255))
            .draw(surface);

        let text_x = x + 6;
        let text_y = y + 7;
        {
            let mut text_target = SurfaceDrawTarget::new(surface);
            let _ = EgText::new(message.as_str(), Point::new(text_x, text_y), message_style)
                .draw(&mut text_target);
        }
    }

    // Draw title text last to avoid surface state issues
    {
        let mut target = SurfaceDrawTarget::new(surface);
        let _ =
            EgText::new(title_text, Point::new(title_x, title_y), title_style).draw(&mut target);
    }
}
