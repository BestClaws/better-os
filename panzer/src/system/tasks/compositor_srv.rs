use alloc::boxed::Box;
use core::fmt::Write;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use crate::system::services::display::{Display, DisplayService};
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::primitives::font::Font;
use gfx::primitives::{CornerRadius, FillStyle, Rectangle};
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;
use swash::zeno::{Bounds, Point};

use micromath::F32Ext;
use crate::ui::themes::pulonia::Palette;  // adjust path if module structure is different

const FONT_DATA_PIXEL: &[u8] = include_bytes!("../../assets/RobotoSlab-SemiBold.ttf");
const FONT_CACHE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789:/-.%+<>#'\"?&()[]{} ";

struct UiFonts {
    title: Font,
    body: Font,
    small: Font,
    time: Font,
}

impl UiFonts {
    fn new(scale: f32) -> Self {
        let title_size = (24.0 / scale).clamp(8.0, 50.0);
        let body_size  = (20.0 / scale).clamp(8.0, 50.0);
        let small_size = (16.0  / scale).clamp(8.0, 50.0);
        let time_size  = (100.0 / scale).clamp(8.0, 50.0);

        let title = Font::builder()
            .data(FONT_DATA_PIXEL)
            .size(title_size)
            .cache(FONT_CACHE)
            .hint(true)
            .build();

        let body = Font::builder()
            .data(FONT_DATA_PIXEL)
            .size(body_size)
            .cache(FONT_CACHE)
            .hint(true)
            .build();

        let small = Font::builder()
            .data(FONT_DATA_PIXEL)
            .size(small_size)
            .cache(FONT_CACHE)
            .hint(true)
            .build();

        let time = Font::builder()
            .data(FONT_DATA_PIXEL)
            .size(time_size)
            .cache(FONT_CACHE)
            .hint(true)
            .build();

        Self {
            title,
            body,
            small,
            time,
        }
    }
}

fn layout_scale(width: f32, height: f32) -> f32 {
    let width_scale  = width  / 205.0;
    let height_scale = height / 251.0;
    width_scale.min(height_scale).clamp(0.45, 1.4)
}

fn draw_text<T: RasterTarget>(
    font: &Font,
    rasterizer: &mut T,
    text: &str,
    x: f32,
    y: f32,
    color: Color,
) {
    if text.is_empty() {
        return;
    }

    let xi = x.round() as i32;
    let yi = y.round() as i32 + font.baseline();
    font.draw_text(rasterizer, text, xi, yi, color);
}

fn render_ui(
    frame_buffer: &mut [u8],
    width: u16,
    height: u16,
    frame_counter: u32,
    pixel_format: PixelFormat,
    scale: u32,
    fonts: &UiFonts,
) {
    frame_buffer.fill(0);

    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, scale, width, height, frame_counter, fonts);
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, scale, width, height, frame_counter, fonts);
        }
    }
}

fn render_ui_with_rasterizer<T: RasterTarget>(
    rasterizer: &mut T,
    scale: u32,
    width: u16,
    height: u16,
    frame_counter: u32,
    fonts: &UiFonts,
) {
    let width_f  = width  as f32;
    let height_f = height as f32;
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width_f, height_f));
    let scale = scale as f32;
    let margin       = 5.0 * scale;
    let status_height = 12.0 * scale;
    let panel_radius  =  5.0 * scale;

    // Status bar container
    let status_x = margin - 4.0;
    let status_y = margin;
    let status_width = (width_f - 2.0 * margin).max(48.0 * scale);



    // Status text
    draw_text(
        &fonts.title,
        rasterizer,
        "Home",
        status_x + 10.0 * scale,
        status_y + 2.0 * scale,
        Palette::TEXT_PRIMARY,
    );




    // Battery indicator
    let battery_height = 7.0 * scale;
    let battery_width  = 14.0 * scale;
    let battery_x = status_x + status_width - battery_width;
    let battery_y = status_y + status_height - battery_height;
    let border = (1.0 * scale).clamp(0.5, 1.5);

    // Outer battery border
    {
        let width = battery_width;
        let height = battery_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(battery_x, battery_y), Point::new(battery_x + width, battery_y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_UMBER))
                .clip(clip);
            let radius = 1.5 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Inner background
    let inner_width  = (battery_width  - 2.0 * border).max(1.0);
    let inner_height = (battery_height - 2.0 * border).max(1.0);
    {
        let x = battery_x + border;
        let y = battery_y + border;
        let width = inner_width;
        let height = inner_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                .fill(FillStyle::solid(Palette::BACKGROUND_SOFT))
                .clip(clip);
            let radius = 1.0 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Battery fill level
    let fill_width = inner_width * 0.78;
    {
        let x = battery_x + border + 1.0 * scale;
        let y = battery_y + border + 0.5 * scale;
        let width = fill_width.max(1.0);
        let height = (inner_height - scale).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_GOLD))
                .clip(clip);
            let radius = 0.8 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Battery nub
    {
        let x = battery_x + battery_width + 1.0 * scale;
        let y = battery_y + (battery_height * 0.35);
        let width  = (1.2 * scale).clamp(0.8, 2.0);
        let height = (battery_height * 0.3).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_UMBER))
                .clip(clip);
            let radius = 0.5 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Time display block
    let time_y = status_y + status_height + 8.0 * scale;

    let total_seconds = frame_counter as u32;
    let hours   = (10 + (total_seconds / 3600) % 12) as u32;
    let minutes = ((24 + (total_seconds / 60)) % 60) as u32;
    let mut time_buf = String::<8>::new();
    let _ = write!(time_buf, "{:02}:{:02}", hours, minutes);

    draw_text(
        &fonts.time,
        rasterizer,
        time_buf.as_str(),
        status_x + status_width / 4.0,
        time_y,
        Palette::ACCENT_GOLD,
    );

    let date_y = time_y + fonts.time.size() + 4.0 * scale;
    draw_text(
        &fonts.body,
        rasterizer,
        "TUE - FEB 04",   // ← updated to match "Current date is February 04, 2026"
        status_x + status_width / 4.0,
        date_y,
        Palette::TEXT_SECONDARY,
    );

    // Quick actions grid
    let quick_top = date_y + fonts.body.size() + 8.0 * scale;
    let mut quick_gap   = 10.0 * scale;
    let mut card_height = 36.0 * scale;
    if height_f <= 170.0 {
        quick_gap   *= 0.75;
        card_height *= 0.85;
    }
    let card_width = ((width_f - 2.0 * margin) - quick_gap).max(40.0 * scale);
    let quick_actions = [
        ("COMMS",  "Secure uplink"),
    ];

    for (idx, (label, subtitle)) in quick_actions.iter().enumerate() {
        let row = idx as f32;

        let card_x = status_x + 14.0;
        let card_y = quick_top + row * (card_height + quick_gap);


        {
            let width = card_width;
            let height = card_height;
            if width > 0.0 && height > 0.0 {
                let mut rect = Rectangle::new()
                    .bounds(Bounds::new(Point::new(card_x, card_y), Point::new(card_x + width, card_y + height)))
                    .fill(FillStyle::Solid(Palette::BACKGROUND_SOFT))
                    .clip(clip);
                let radius = 6.0 * scale;
                if radius > 0.0 {
                    rect = rect.corner_radii(CornerRadius::new(radius, radius));
                }
                rect.draw(rasterizer);
            }
        }

        // Left accent line
        {
            let x = card_x + 6.0 * scale;
            let y = card_y + 6.0 * scale;
            let width  = 3.0 * scale;
            let height = card_height - 12.0 * scale;
            if width > 0.0 && height > 0.0 {
                let mut rect = Rectangle::new()
                    .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                    .fill(FillStyle::solid(if idx == 0 { Palette::ACCENT_GOLD } else { Palette::ACCENT_UMBER }))
                    .clip(clip);
                let radius = 1.5 * scale;
                if radius > 0.0 {
                    rect = rect.corner_radii(CornerRadius::new(radius, radius));
                }
                rect.draw(rasterizer);
            }
        }

        draw_text(&fonts.body,  rasterizer, label,    card_x + 12.0 * scale, card_y +  6.0 * scale, Palette::TEXT_DARK);
        draw_text(&fonts.small, rasterizer, subtitle, card_x + 12.0 * scale, card_y + 17.0 * scale, Palette::TEXT_DARK);
    }





}

/// Compositor service task - handles UI rendering, animations, and display upkeep.
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!("Starting compositor service");

    // Negotiate display capabilities first
    let display_facade: Display = DisplayService::new(display).initialize().await;
    let resolution = display_facade.resolution();
    let negotiated_pixel_format = display_facade.pixel_format();

    // Initialize display hardware
    {
        let mut display_lock = display.lock().await;
        display_lock.init().await;
        display_lock.set_brightness(0xFF).await;
        info!("Display initialized: brightness=100%");
    }

    let width  = resolution.logical.width  as usize;
    let height = resolution.logical.height as usize;
    let scale = resolution.scale;
    let width_u16  = width  as u16;
    let height_u16 = height as u16;

    let buffer_size = negotiated_pixel_format.framebuffer_size(width as u32, height as u32);
    let mut buffer = alloc::vec![0u8; buffer_size];

    let fonts = UiFonts::new(scale as f32);

    info!(
        "Compositor UI loop active {:?} @ {}x{}",
        negotiated_pixel_format, width, height
    );

    let mut frame_counter = 0u32;
    loop {
        let frame_start = Instant::now();

        let render_start = Instant::now();
        render_ui(
            &mut buffer,
            width_u16,
            height_u16,
            frame_counter,
            negotiated_pixel_format,
            scale,
            &fonts,
        );
        let render_time = render_start.elapsed().as_micros();

        let draw_start = Instant::now();
        display_facade.draw_full(&buffer).await;
        let flush_time = draw_start.elapsed().as_micros();

        let frame_time = frame_start.elapsed().as_micros();

        info!(
            "frame={} render={}us flush={}us total={}us",
            frame_counter, render_time, flush_time, frame_time
        );

        frame_counter = frame_counter.wrapping_add(1);
        Timer::after(Duration::from_millis(33)).await;   // ~30 fps target
    }
}