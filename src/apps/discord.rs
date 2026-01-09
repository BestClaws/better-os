use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::{RoundedRect, Shape, SurfaceDrawTarget};
use crate::libs::http_bridge::{HttpBridgeError, HttpClient};
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
const POLL_INTERVAL_MS: u64 = 5_000;
const POLL_PATH: &str = "/get";
const MAX_DISPLAY_CHARS: usize = 36;

#[task]
pub async fn discord_app(ctx: AppContext) {
    info!("Starting Discord bridge app");
    let client = HttpClient::new();
    let mut messages = [
        placeholder_message(),
        placeholder_message(),
        placeholder_message(),
    ];
    let mut connected = false;
    let mut last_error: Option<HttpBridgeError> = None;
    let mut ticker = Ticker::every(Duration::from_millis(POLL_INTERVAL_MS));
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            if ctx.is_focused().await {
                ctx.draw(|surface| draw_interface(surface, &messages, connected))
                    .await;
                needs_redraw = false;
            }
        }

        ticker.next().await;

        match client.get(POLL_PATH).send().await {
            Ok(response) => {
                if !connected {
                    connected = true;
                    needs_redraw = true;
                }
                if update_messages(&mut messages, response.body()) {
                    needs_redraw = true;
                }
                last_error = None;
            }
            Err(err) => {
                if last_error != Some(err) {
                    warn!("discord poll failed: {}", err);
                    last_error = Some(err);
                }
                if connected {
                    connected = false;
                    needs_redraw = true;
                }
            }
        }
    }
}

fn update_messages(messages: &mut [String<MESSAGE_CAPACITY>; 3], body: &[u8]) -> bool {
    let text = core::str::from_utf8(body).unwrap_or("");
    let mut changed = false;

    for (idx, slot) in messages.iter_mut().enumerate() {
        let line = text.lines().nth(idx).unwrap_or("...");
        let sanitized = sanitize_line(line);
        if slot.as_str() != sanitized.as_str() {
            slot.clear();
            slot.push_str(sanitized.as_str()).ok();
            changed = true;
        }
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
    
    // Clean gradient background
    surface.fill_rect(0, 0, width, height, Rgba8888::rgba(24, 25, 28, 255));

    // Simple, clean header
    let header_height = 22;
    surface.fill_rect(0, 0, width, header_height, Rgba8888::rgba(32, 34, 37, 255));
    surface.fill_rect(0, header_height, width, 1, Rgba8888::rgba(0, 0, 0, 60));

    let title_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(242, 243, 245));
    let message_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(219, 222, 225));

    // Centered title
    let title_text = "Discord";
    let title_width = title_text.len() as i32 * FONT_5X8.character_size.width as i32;
    let title_x = (width - title_width) / 2;
    let title_y = 7;

    {
        let mut target = SurfaceDrawTarget::new(surface);
        let _ =
            EgText::new(title_text, Point::new(title_x, title_y), title_style).draw(&mut target);
    }

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
}
