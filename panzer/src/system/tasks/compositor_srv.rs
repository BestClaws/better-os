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
        let title_size = (11.0 * scale).clamp(6.0, 13.0);
        let body_size = (8.0 * scale).clamp(5.0, 10.0);
        let small_size = (6.0 * scale).clamp(4.0, 8.0);
        let time_size = (20.0 * scale).clamp(12.0, 26.0);

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

struct Palette;

impl Palette {
    fn background_dark() -> Color {
        Color::rgba(0x1a, 0x17, 0x14, 0xff)
    }

    fn background_deep() -> Color {
        Color::rgba(0x1f, 0x1c, 0x18, 0xff)
    }

    fn background_panel() -> Color {
        Color::rgba(0x25, 0x21, 0x1c, 0xff)
    }

    fn background_soft() -> Color {
        Color::rgba(0x2b, 0x26, 0x21, 0xff)
    }

    fn accent_gold() -> Color {
        Color::rgba(0xc8, 0xa7, 0x60, 0xff)
    }

    fn accent_rose() -> Color {
        Color::rgba(0xb8, 0x49, 0x5d, 0xff)
    }

    fn accent_umber() -> Color {
        Color::rgba(0x8b, 0x73, 0x55, 0xff)
    }

    fn accent_copper() -> Color {
        Color::rgba(0x8b, 0x4a, 0x52, 0xff)
    }

    fn progress_back() -> Color {
        Color::rgba(0x3a, 0x34, 0x2b, 0xff)
    }

    fn text_primary() -> Color {
        Color::rgba(0xe8, 0xdc, 0xc8, 0xff)
    }

    fn text_secondary() -> Color {
        Color::rgba(0xa8, 0x98, 0x78, 0xff)
    }

    fn text_muted() -> Color {
        Color::rgba(0x78, 0x68, 0x48, 0xff)
    }
}

fn layout_scale(width: f32, height: f32) -> f32 {
    let width_scale = width / 205.0;
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
    fonts: &UiFonts,
) {
    frame_buffer.fill(0);

    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, width, height, frame_counter, fonts);
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, width, height, frame_counter, fonts);
        }
    }
}

fn render_ui_with_rasterizer<T: RasterTarget>(
    rasterizer: &mut T,
    width: u16,
    height: u16,
    frame_counter: u32,
    fonts: &UiFonts,
) {
    let width_f = width as f32;
    let height_f = height as f32;
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(width_f, height_f));
    let scale = layout_scale(width_f, height_f);

    let margin = 12.0 * scale;
    let status_height = 24.0 * scale;
    let panel_radius = 7.0 * scale;

    // Background gradient
    {
        let width = width_f;
        let height = height_f;
        if width > 0.0 && height > 0.0 {
            Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(0.0, 0.0),
                    Point::new(width, height),
                ))
                .fill(FillStyle::vertical_gradient([
                    (Palette::background_dark(), 0),
                    (Palette::background_deep(), 140),
                    (Palette::background_panel(), 255),
                ]))
                .clip(clip)
                .draw(rasterizer);
        }
    }

    // Status bar container
    let status_x = margin;
    let status_y = margin;
    let status_width = (width_f - 2.0 * margin).max(48.0 * scale);
    {
        let width = status_width;
        let height = status_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(status_x, status_y),
                    Point::new(status_x + width, status_y + height),
                ))
                .fill(FillStyle::horizontal_gradient([
                    (Palette::background_panel(), 0),
                    (Palette::background_soft(), 128),
                    (Palette::background_deep(), 255),
                ]))
                .clip(clip);
            if panel_radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(panel_radius, panel_radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Status accent strip
    let accent_strip_width = (4.0 * scale).clamp(2.0, 6.0);
    {
        let x = status_x + 4.0 * scale;
        let y = status_y + 4.0 * scale;
        let width = accent_strip_width;
        let height = status_height - 8.0 * scale;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(x, y),
                    Point::new(x + width, y + height),
                ))
                .fill(FillStyle::solid(Palette::accent_copper()))
                .clip(clip);
            let radius = accent_strip_width.min(3.0 * scale);
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Status text
    draw_text(
        &fonts.title,
        rasterizer,
        "Better OS",
        status_x + 12.0 * scale,
        status_y + 3.0 * scale,
        Palette::text_primary(),
    );

    draw_text(
        &fonts.small,
        rasterizer,
        "Flight Deck Control",
        status_x + 12.0 * scale,
        status_y + 12.5 * scale,
        Palette::text_secondary(),
    );

    let mut frame_buf = String::<16>::new();
    let _ = write!(frame_buf, "Frame {:05}", frame_counter);
    draw_text(
        &fonts.small,
        rasterizer,
        frame_buf.as_str(),
        status_x + status_width - 70.0 * scale,
        status_y + 12.0 * scale,
        Palette::text_muted(),
    );

    // Battery indicator using layered rectangles
    let battery_height = 7.0 * scale;
    let battery_width = 16.0 * scale;
    let battery_x = status_x + status_width - battery_width - 12.0 * scale;
    let battery_y = status_y + (status_height - battery_height) * 0.5;
    let border = (1.0 * scale).clamp(0.5, 1.5);

    {
        let width = battery_width;
        let height = battery_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(battery_x, battery_y),
                    Point::new(battery_x + width, battery_y + height),
                ))
                .fill(FillStyle::solid(Palette::accent_umber()))
                .clip(clip);
            let radius = 1.5 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    let inner_width = (battery_width - 2.0 * border).max(1.0);
    let inner_height = (battery_height - 2.0 * border).max(1.0);
    {
        let x = battery_x + border;
        let y = battery_y + border;
        let width = inner_width;
        let height = inner_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(x, y),
                    Point::new(x + width, y + height),
                ))
                .fill(FillStyle::solid(Palette::background_soft()))
                .clip(clip);
            let radius = 1.0 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    let fill_width = inner_width * 0.78;
    {
        let x = battery_x + border + 1.0 * scale;
        let y = battery_y + border + 0.5 * scale;
        let width = fill_width.max(1.0);
        let height = (inner_height - scale).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(x, y),
                    Point::new(x + width, y + height),
                ))
                .fill(FillStyle::solid(Palette::accent_gold()))
                .clip(clip);
            let radius = 0.8 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    {
        let x = battery_x + battery_width + 1.0 * scale;
        let y = battery_y + (battery_height * 0.35);
        let width = (1.2 * scale).clamp(0.8, 2.0);
        let height = (battery_height * 0.3).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(x, y),
                    Point::new(x + width, y + height),
                ))
                .fill(FillStyle::solid(Palette::accent_umber()))
                .clip(clip);
            let radius = 0.5 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Time display block
    let label_y = status_y + status_height + 8.0 * scale;
    draw_text(
        &fonts.small,
        rasterizer,
        "Command Uplink",
        status_x,
        label_y,
        Palette::text_secondary(),
    );

    let time_y = label_y + 6.0 * scale;

    let total_seconds = frame_counter as u32;
    let hours = (10 + (total_seconds / 3600) % 12) as u32;
    let minutes = ((24 + (total_seconds / 60)) % 60) as u32;
    let mut time_buf = String::<8>::new();
    let _ = write!(time_buf, "{:02}:{:02}", hours, minutes);

    draw_text(
        &fonts.time,
        rasterizer,
        time_buf.as_str(),
        status_x,
        time_y,
        Palette::accent_gold(),
    );

    let date_y = time_y + fonts.time.size() + 4.0 * scale;
    draw_text(
        &fonts.body,
        rasterizer,
        "TUE - FEB 03",
        status_x,
        date_y,
        Palette::text_secondary(),
    );

    // Quick actions grid
    let quick_top = date_y + fonts.body.size() + 8.0 * scale;
    let mut quick_gap = 10.0 * scale;
    let mut card_height = 36.0 * scale;
    if height_f <= 170.0 {
        quick_gap *= 0.75;
        card_height *= 0.85;
    }
    let card_width = ((width_f - 2.0 * margin) - quick_gap).max(40.0 * scale) / 2.0;
    let quick_actions = [
        ("COMMS", "Secure uplink"),
        ("TRACK", "Orbital path"),
        ("SENSORS", "Calibration"),
        ("LOGS", "Mission events"),
    ];

    for (idx, (label, subtitle)) in quick_actions.iter().enumerate() {
        let col = (idx % 2) as f32;
        let row = (idx / 2) as f32;

        let card_x = status_x + col * (card_width + quick_gap);
        let card_y = quick_top + row * (card_height + quick_gap);

        let fill = if idx == 0 {
            FillStyle::vertical_gradient([
                (Palette::accent_rose(), 0),
                (Palette::accent_copper(), 128),
                (Palette::background_panel(), 255),
            ])
        } else {
            FillStyle::vertical_gradient([
                (Palette::background_panel(), 0),
                (Palette::background_soft(), 128),
                (Palette::background_soft(), 255),
            ])
        };

        {
            let width = card_width;
            let height = card_height;
            if width > 0.0 && height > 0.0 {
                let mut rect = Rectangle::new()
                    .bounds(Bounds::new(
                        Point::new(card_x, card_y),
                        Point::new(card_x + width, card_y + height),
                    ))
                    .fill(fill)
                    .clip(clip);
                let radius = 6.0 * scale;
                if radius > 0.0 {
                    rect = rect.corner_radii(CornerRadius::new(radius, radius));
                }
                rect.draw(rasterizer);
            }
        }

        {
            let x = card_x + 6.0 * scale;
            let y = card_y + 6.0 * scale;
            let width = 3.0 * scale;
            let height = card_height - 12.0 * scale;
            if width > 0.0 && height > 0.0 {
                let mut rect = Rectangle::new()
                    .bounds(Bounds::new(
                        Point::new(x, y),
                        Point::new(x + width, y + height),
                    ))
                    .fill(FillStyle::solid(if idx == 0 {
                        Palette::accent_gold()
                    } else {
                        Palette::accent_umber()
                    }))
                    .clip(clip);
                let radius = 1.5 * scale;
                if radius > 0.0 {
                    rect = rect.corner_radii(CornerRadius::new(radius, radius));
                }
                rect.draw(rasterizer);
            }
        }

        draw_text(
            &fonts.body,
            rasterizer,
            label,
            card_x + 12.0 * scale,
            card_y + 6.0 * scale,
            Palette::text_primary(),
        );

        draw_text(
            &fonts.small,
            rasterizer,
            subtitle,
            card_x + 12.0 * scale,
            card_y + 17.0 * scale,
            Palette::text_secondary(),
        );
    }

    // Stats card
    let mut stats_height = 52.0 * scale;
    if height_f <= 170.0 {
        stats_height *= 0.85;
    }
    let quick_second_row_y = quick_top + card_height + quick_gap;
    let quick_bottom = quick_second_row_y + card_height;
    let bottom_margin = 10.0 * scale;

    let mut stats_top = quick_bottom + 8.0 * scale;
    if stats_top + stats_height + bottom_margin > height_f {
        stats_top = (height_f - stats_height - bottom_margin).max(quick_bottom + 4.0 * scale);
    }
    if stats_top + stats_height + bottom_margin > height_f {
        stats_height = (height_f - stats_top - bottom_margin).max(30.0 * scale);
    }

    {
        let width = status_width;
        let height = stats_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(status_x, stats_top),
                    Point::new(status_x + width, stats_top + height),
                ))
                .fill(FillStyle::horizontal_gradient([
                    (Palette::background_panel(), 0),
                    (Palette::background_soft(), 128),
                    (Palette::background_deep(), 255),
                ]))
                .clip(clip);
            let radius = 6.0 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    {
        let x = status_x + 6.0 * scale;
        let y = stats_top + 8.0 * scale;
        let width = 3.0 * scale;
        let height = stats_height - 16.0 * scale;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(x, y),
                    Point::new(x + width, y + height),
                ))
                .fill(FillStyle::solid(Palette::accent_gold()))
                .clip(clip);
            let radius = 1.5 * scale;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    draw_text(
        &fonts.body,
        rasterizer,
        "POWER SYSTEMS",
        status_x + 14.0 * scale,
        stats_top + 6.0 * scale,
        Palette::text_primary(),
    );

    draw_text(
        &fonts.small,
        rasterizer,
        "Peak output stable",
        status_x + 14.0 * scale,
        stats_top + fonts.body.size() + 10.0 * scale,
        Palette::text_secondary(),
    );

    let progress_x = status_x + 14.0 * scale;
    let progress_height = 6.0 * scale;
    let mut progress_y = stats_top + stats_height - (progress_height + 16.0 * scale);
    let lower_bound = stats_top + fonts.body.size() + 14.0 * scale;
    let upper_bound = stats_top + stats_height - progress_height - 6.0 * scale;
    let upper_bound = upper_bound.max(lower_bound);
    progress_y = progress_y.clamp(lower_bound, upper_bound);
    let progress_width = status_width - 28.0 * scale;

    {
        let width = progress_width;
        let height = progress_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(progress_x, progress_y),
                    Point::new(progress_x + width, progress_y + height),
                ))
                .fill(FillStyle::solid(Palette::progress_back()))
                .clip(clip);
            let radius = progress_height * 0.5;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    let cycle = (frame_counter % 180) as f32 / 180.0;
    let ramp = if cycle < 0.5 {
        cycle * 2.0
    } else {
        (1.0 - cycle) * 2.0
    };
    let progress_ratio = 0.42 + 0.48 * ramp;
    let active_width = (progress_width * progress_ratio).min(progress_width);

    {
        let width = active_width;
        let height = progress_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(
                    Point::new(progress_x, progress_y),
                    Point::new(progress_x + width, progress_y + height),
                ))
                .fill(FillStyle::horizontal_gradient([
                    (Palette::accent_rose(), 0),
                    (Palette::accent_gold(), 128),
                    (Palette::accent_gold(), 255),
                ]))
                .clip(clip);
            let radius = progress_height * 0.5;
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    let mut status_buf = String::<24>::new();
    let _ = write!(status_buf, "Output {:>3}% Nominal", (progress_ratio * 100.0) as u32);
    draw_text(
        &fonts.small,
        rasterizer,
        status_buf.as_str(),
        status_x + 14.0 * scale,
        progress_y + progress_height + 4.0 * scale,
        Palette::text_secondary(),
    );

    // Footer note
    let footer_y = stats_top + stats_height + bottom_margin;
    if footer_y + fonts.small.baseline() as f32 <= height_f {
        draw_text(
            &fonts.small,
            rasterizer,
            "Style guide applied - Rect + Font only",
            status_x,
            footer_y,
            Palette::text_muted(),
        );
    }
}

/// Compositor service task - handles UI rendering, animations, and display upkeep.
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!("Starting compositor service");

    // Negotiate display capabilities first (before hardware init)
    let display_facade: Display = DisplayService::new(display).initialize().await;
    let resolution = display_facade.resolution();
    let negotiated_pixel_format = display_facade.pixel_format();

    // Initialize display hardware with negotiated settings
    let mut display_lock = display.lock().await;
    display_lock.init().await;
    display_lock.set_brightness(0xFF).await;
    info!("Display initialized: brightness=100%");
    drop(display_lock);

    let width = resolution.logical.width as usize;
    let height = resolution.logical.height as usize;
    let width_u16 = width as u16;
    let height_u16 = height as u16;

    // Allocate buffer based on negotiated pixel format
    let buffer_size = negotiated_pixel_format.framebuffer_size(width as u32, height as u32);
    let mut buffer = alloc::vec![0u8; buffer_size];

    let fonts = UiFonts::new(layout_scale(width as f32, height as f32));

    info!(
        "Compositor UI loop active {:?} @ {}x{}",
        negotiated_pixel_format,
        width,
        height
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
            &fonts,
        );
        let render_time = render_start.elapsed().as_micros();

        let draw_start = Instant::now();
        display_facade.draw_full(&buffer).await;
        let flush_time = draw_start.elapsed().as_micros();

        let frame_time = frame_start.elapsed().as_micros();

        info!(
            "frame={} render={}us flush={}us total={}us",
            frame_counter,
            render_time,
            flush_time,
            frame_time
        );

        frame_counter = frame_counter.wrapping_add(1);
        Timer::after(Duration::from_millis(3)).await;
    }
}
