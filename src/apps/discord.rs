use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::libs::gfx::{Circle, RoundedRect, Shape, SurfaceDrawTarget};
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
    surface.fill_rect(0, 0, width, height, Rgba8888::rgba(10, 13, 20, 255));

    let cx = width / 2;
    let cy = height / 2;
    let radius = (width.min(height) / 2) - 2;

    Circle::new(cx, cy, radius)
        .fill_radial(
            Rgba8888::rgba(56, 62, 82, 255),
            Rgba8888::rgba(18, 22, 30, 255),
        )
        .stroke(2, Rgba8888::rgba(88, 101, 242, 220))
        .stroke_alpha(180)
        .draw(surface);

    let title_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(214, 218, 255));
    let status_style = MonoTextStyle::new(&FONT_4X6, Rgb888::new(172, 180, 198));
    let message_style = MonoTextStyle::new(&FONT_5X8, Rgb888::new(235, 238, 242));

    let title_text = "Discord";
    let title_width = title_text.len() as i32 * FONT_5X8.character_size.width as i32;
    let title_x = cx - title_width / 2;
    let title_y = cy - radius + 12;

    {
        let mut target = SurfaceDrawTarget::new(surface);
        let _ =
            EgText::new(title_text, Point::new(title_x, title_y), title_style).draw(&mut target);
    }

    let status_text = if connected { "online" } else { "offline" };
    let status_char_w = FONT_4X6.character_size.width as i32;
    let status_char_h = FONT_4X6.character_size.height as i32;
    let status_width = status_text.len() as i32 * status_char_w;
    let status_box_width = status_width + 12;
    let status_box_height = status_char_h + 6;
    let status_box_x = cx - status_box_width / 2;
    let status_box_y = title_y + FONT_5X8.character_size.height as i32 + 6;

    RoundedRect::new(
        status_box_x,
        status_box_y,
        status_box_width,
        status_box_height,
        8,
        8,
        8,
        8,
    )
    .fill_linear_h(
        Rgba8888::rgba(96, 108, 224, 210),
        Rgba8888::rgba(68, 74, 116, 210),
    )
    .stroke(1, Rgba8888::rgba(28, 32, 48, 200))
    .draw(surface);

    {
        let mut target = SurfaceDrawTarget::new(surface);
        let text_x = cx - status_width / 2;
        let text_y = status_box_y + (status_box_height - status_char_h) / 2 + 1;
        let _ =
            EgText::new(status_text, Point::new(text_x, text_y), status_style).draw(&mut target);
    }

    let content_top = status_box_y + status_box_height + 4;
    let content_diameter = radius * 2;
    let bubble_width = (content_diameter - 48).max(40);
    let bubble_height = 18;
    let bubble_spacing = 5;
    let left_x = cx - radius + 14;
    let right_x = cx + radius - bubble_width - 14;

    for (idx, message) in messages.iter().enumerate() {
        let align_right = idx % 2 == 1;
        let x = if align_right { right_x } else { left_x };
        let y = content_top + idx as i32 * (bubble_height + bubble_spacing);

        RoundedRect::new(x, y, bubble_width, bubble_height, 10, 10, 10, 10)
            .fill_linear_h(
                if align_right {
                    Rgba8888::rgba(70, 78, 140, 235)
                } else {
                    Rgba8888::rgba(58, 64, 92, 230)
                },
                Rgba8888::rgba(38, 42, 58, 220),
            )
            .stroke(1, Rgba8888::rgba(25, 28, 40, 255))
            .draw(surface);

        let text_x = x + 8;
        let text_y = y + 5;
        {
            let mut text_target = SurfaceDrawTarget::new(surface);
            let _ = EgText::new(message.as_str(), Point::new(text_x, text_y), message_style)
                .draw(&mut text_target);
        }
    }
}
