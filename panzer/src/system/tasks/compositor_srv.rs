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
use gfx::primitives::{CornerRadius, Edge, FillStyle, Rectangle, StrokeStyle};
use gfx::rasterizer::RasterTarget;
use gfx::rgb565::Rgb565Rasterizer;
use swash::zeno::{Bounds, Point};

use micromath::F32Ext;
use swash::zeno;
use swash::zeno::Style::Stroke;
use crate::ui::themes::pulonia::Palette;  // adjust path if module structure is different

/// Baseline DPI for 1:1 device pixel ratio (CSS reference pixel density)
const BASELINE_DPI: f32 = 160.0;

const FONT_DATA_PIXEL: &[u8] = include_bytes!("../../assets/Endfield.ttf");
const FONT_CACHE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789:/-.%+<>#'\"?&()[]{} ";

// Starfield configuration
const STAR_COUNT: usize = 80;

#[derive(Copy, Clone)]
struct Star {
    x: f32,        // Normalized position (-1 to 1)
    y: f32,        // Normalized position (-1 to 1)
    z: f32,        // Depth (0 to 1, closer stars have higher z)
    brightness: u8, // Star brightness (0-255)
}

struct UiFonts {
    title: Font,
    body: Font,
    small: Font,
    time: Font,
}

impl UiFonts {
    fn new(scale: f32) -> Self {
        let px = |v: f32| v * scale;
        let title_size = px(16.0);
        let body_size  = px(14.0);
        let small_size = px(12.0);
        let time_size  = px(50.0);

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

fn generate_stars(frame_counter: u32) -> [Star; STAR_COUNT] {
    let mut stars = [Star { x: 0.0, y: 0.0, z: 0.0, brightness: 255 }; STAR_COUNT];
    
    // Smooth movement speed
    let time = frame_counter as f32 * 0.036; // Increased for faster star movement
    
    for i in 0..STAR_COUNT {
        // Use only the star index for position (not frame counter)
        // This makes star positions persistent across frames
        let seed = (i as u32).wrapping_mul(73856093);
        let seed2 = (i as u32).wrapping_mul(83492791);
        
        // Generate fixed normalized coordinates (-1 to 1) per star
        let x = ((seed % 2000) as f32 / 1000.0) - 1.0;
        let y = ((seed2 % 2000) as f32 / 1000.0) - 1.0;
        
        // Each star has a fixed initial depth and speed offset
        let base_z = ((seed >> 16) % 1000) as f32 / 1000.0;
        let speed_offset = ((seed >> 8) % 100) as f32 / 100.0; // Slight speed variation per star
        
        // Animate depth over time (stars move toward camera)
        let z = ((base_z + time * (0.8 + speed_offset * 0.4)) % 1.0);
        
        // Brightness increases as stars get closer (higher z)
        let brightness = (50 + (z * 205.0) as u32).min(255) as u8;
        
        stars[i] = Star { x, y, z, brightness };
    }
    
    stars
}

fn render_starfield<T: RasterTarget>(
    rasterizer: &mut T,
    width: u16,
    height: u16,
    frame_counter: u32,
) {
    let stars = generate_stars(frame_counter);
    let width_f = width as f32;
    let height_f = height as f32;
    let center_x = width_f / 2.0;
    let center_y = height_f / 2.0;
    
    for star in &stars {
        // Project star position from center based on depth
        // Stars further away (low z) are closer to center
        // Stars closer (high z) are further from center
        // Reduced perspective multiplier for slower apparent motion
        let perspective = 1.0 + star.z * 1.5; // Reduced from 2.0 to 1.5
        
        let screen_x = center_x + (star.x * center_x * perspective);
        let screen_y = center_y + (star.y * center_y * perspective);
        
        // Only draw stars within screen bounds
        if screen_x >= 0.0 && screen_x < width_f && screen_y >= 0.0 && screen_y < height_f {
            let x = screen_x as u16;
            let y = screen_y as u16;
            
            // Star size increases with depth (closer stars are bigger)
            let size = if star.z > 0.8 {
                2 // Larger stars when very close
            } else {
                1 // Single pixel for distant stars
            };
            
            let color = Color::rgba(star.brightness, star.brightness, star.brightness, 255);
            
            // Draw star pixel(s) using fill_solid_rect for single pixels
            rasterizer.fill_solid_rect(x, y, 1, 1, color);
            
            if size > 1 && x > 0 && y > 0 && x < width - 1 && y < height - 1 {
                // Draw a small cross for larger stars
                rasterizer.fill_solid_rect(x + 1, y, 1, 1, color);
                rasterizer.fill_solid_rect(x, y + 1, 1, 1, color);
            }
        }
    }
}


fn render_ui(
    frame_buffer: &mut [u8],
    width: u16,
    height: u16,
    frame_counter: u32,
    pixel_format: PixelFormat,
    device_pixel_ratio: f32,
    fonts: &UiFonts,
    snake_angle: f32,
    gradient_angle: f32,
) {
    frame_buffer.fill(0);

    match pixel_format {
        PixelFormat::Gray4 => {
            let mut rasterizer = Luma4Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, device_pixel_ratio, width, height, frame_counter, fonts, snake_angle, gradient_angle);
        }
        PixelFormat::Rgb565 => {
            let mut rasterizer = Rgb565Rasterizer::new(frame_buffer, width, height);
            render_ui_with_rasterizer(&mut rasterizer, device_pixel_ratio, width, height, frame_counter, fonts, snake_angle, gradient_angle);
        }
    }
}

fn render_ui_with_rasterizer<T: RasterTarget>(
    rasterizer: &mut T,
    device_pixel_ratio: f32,
    width: u16,
    height: u16,
    frame_counter: u32,
    fonts: &UiFonts,
    snake_angle: f32,
    gradient_angle: f32,
) {
    let width_f  = width  as f32;
    let height_f = height as f32;
    let clip = Bounds::new(Point::new(0.0, 0.0), Point::new(1000., 1000.));
    let px = |v: f32| v * device_pixel_ratio;

    // Grayscale gradient colors for better performance on gray4
    // Equal delta of 60 between each step for consistent gradient
    let color_white = Color::rgba(255, 255, 255, 255);      // White - brightest
    let color_light = Color::rgba(195, 195, 195, 255);      // Light gray (255 - 60)
    let color_mid = Color::rgba(135, 135, 135, 255);        // Mid gray (195 - 60)
    let color_dark = Color::rgba(75, 75, 75, 255);          // Dark gray (135 - 60)
    let color_black = Color::rgba(0, 0, 0, 255);            // Complete black for snake background
    let color_transparent = Color::rgba(0, 0, 0, 0);        // Transparent
    
    // Helper function to interpolate colors
    let interpolate_color = |c1: Color, c2: Color, t: f32| -> Color {
        let r = (c1.r() as f32 + (c2.r() as f32 - c1.r() as f32) * t) as u8;
        let g = (c1.g() as f32 + (c2.g() as f32 - c1.g() as f32) * t) as u8;
        let b = (c1.b() as f32 + (c2.b() as f32 - c1.b() as f32) * t) as u8;
        let a = (c1.a() as f32 + (c2.a() as f32 - c1.a() as f32) * t) as u8;
        Color::rgba(r, g, b, a)
    };
    
    // Snake gradient for bigger rect - creates a "snake" segment that rotates around
    // The snake has a width/length and the rest is completely black
    let get_snake_edge_gradient = |edge_start_angle: f32| -> (Color, Color) {
        // Snake parameters
        let snake_length = 60.0; // How many degrees the snake occupies
        let snake_head_angle = snake_angle;
        
        // Calculate edge position relative to snake
        let edge_angle = edge_start_angle;
        let edge_end_angle = edge_start_angle + 90.0;
        
        // Check if this edge overlaps with the snake
        let angle_diff = |a1: f32, a2: f32| -> f32 {
            let mut diff = (a1 - a2 + 180.0) % 360.0 - 180.0;
            if diff < -180.0 { diff += 360.0; }
            diff.abs()
        };
        
        let start_color = {
            let diff = angle_diff(edge_angle, snake_head_angle);
            if diff < snake_length / 2.0 {
                // Within the snake - fade from white (brightest) to black (darkest)
                let t = diff / (snake_length / 2.0);
                interpolate_color(color_white, color_black, t)
            } else {
                color_black // Rest of rect is completely black
            }
        };
        
        let end_color = {
            let diff = angle_diff(edge_end_angle, snake_head_angle);
            if diff < snake_length / 2.0 {
                let t = diff / (snake_length / 2.0);
                interpolate_color(color_white, color_black, t)
            } else {
                color_black // Rest of rect is completely black
            }
        };
        
        (start_color, end_color)
    };

    // BORDERS RECT - Draw first before everything with snake gradient
    // Positioned with 1% margin from top-left and 1% margin from bottom-right
    let big_margin = 0.01; // 1% margin
    let big_x = width_f * big_margin;
    let big_y = height_f * big_margin;
    let big_width = width_f * (1.0 - 2.0 * big_margin);
    let big_height = height_f * (1.0 - 2.0 * big_margin);
    
    let (top_start, top_end) = get_snake_edge_gradient(0.0);
    let (right_start, right_end) = get_snake_edge_gradient(90.0);
    let (bottom_start, bottom_end) = get_snake_edge_gradient(180.0);
    let (left_start, left_end) = get_snake_edge_gradient(270.0);
    
    let mut rect = Rectangle::new()
        .edge(Edge::Top, StrokeStyle::from_stroke(zeno::Stroke::new(1.0))
            .horizontal_gradient([(top_start, 0), (top_end, 255)]))
        .edge(Edge::Right, StrokeStyle::from_stroke(zeno::Stroke::new(1.0))
            .vertical_gradient([(right_start, 0), (right_end, 255)]))
        .edge(Edge::Bottom, StrokeStyle::from_stroke(zeno::Stroke::new(1.0))
            .horizontal_gradient([(bottom_end, 0), (bottom_start, 255)]))
        .edge(Edge::Left, StrokeStyle::from_stroke(zeno::Stroke::new(1.0))
            .vertical_gradient([(left_end, 0), (left_start, 255)]))
        .corner_radii(CornerRadius::new(64., 64.))
        .bounds(Bounds::new(Point::new(big_x, big_y), Point::new(big_x + big_width, big_y + big_height)))
        .clip(clip);
    rect.draw(rasterizer);

    // Render starfield background
    render_starfield(rasterizer, width, height, frame_counter);

    // Status elements positioned at 10% from top and sides
    let home_x = width_f * 0.10;
    let home_y = height_f * 0.10;

    // Status text
    draw_text(
        &fonts.title,
        rasterizer,
        "HOME",
        home_x,
        home_y,
        Palette::TEXT_PRIMARY,
    );




    // Battery indicator - positioned at top-right
    let battery_height = px(7.0);
    let battery_width  = px(14.0);
    let battery_x = width_f * 0.90 - battery_width;
    let battery_y = height_f * 0.10;
    let border = px(1.0).clamp(0.5, 1.5);

    // Outer battery border
    {
        let width = battery_width;
        let height = battery_height;
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(battery_x, battery_y), Point::new(battery_x + width, battery_y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_UMBER))
                .clip(clip);
            let radius = px(1.5);
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
            let radius = px(1.0);
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Battery fill level
    let fill_width = inner_width * 0.78;
    {
        let x = battery_x + border + px(1.0);
        let y = battery_y + border + px(0.5);
        let width = fill_width.max(1.0);
        let height = (inner_height - px(1.0)).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_GOLD))
                .clip(clip);
            let radius = px(0.8);
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Battery nub
    {
        let x = battery_x + battery_width + px(1.0);
        let y = battery_y + (battery_height * 0.35);
        let width  = px(1.2).clamp(0.8, 2.0);
        let height = (battery_height * 0.3).max(1.0);
        if width > 0.0 && height > 0.0 {
            let mut rect = Rectangle::new()
                .bounds(Bounds::new(Point::new(x, y), Point::new(x + width, y + height)))
                .fill(FillStyle::solid(Palette::ACCENT_UMBER))
                .clip(clip);
            let radius = px(0.5);
            if radius > 0.0 {
                rect = rect.corner_radii(CornerRadius::new(radius, radius));
            }
            rect.draw(rasterizer);
        }
    }

    // Time display block - centered at 25% viewport height
    let time_y = height_f * 0.30;

    let total_seconds = frame_counter as u32;
    let hours   = (10 + (total_seconds / 3600) % 12) as u32;
    let minutes = ((24 + (total_seconds / 60)) % 60) as u32;
    let mut time_buf = String::<8>::new();
    let _ = write!(time_buf, "{:02}:{:02}", hours, minutes);

    let (time_width, time_height) = fonts.time.text_dimensions(time_buf.as_str());
    let time_x = (width_f - time_width) / 2.0;
    let time_y_centered = time_y - time_height / 2.0;
    draw_text(
        &fonts.time,
        rasterizer,
        time_buf.as_str(),
        time_x,
        time_y_centered,
        Palette::ACCENT_GOLD,
    );

    // Date just below clock, centered
    let date_text = "TUE - FEB 04";
    let (date_width, date_height) = fonts.body.text_dimensions(date_text);
    let date_x = (width_f - date_width) / 2.0;
    let date_y_top = time_y_centered + time_height + px(4.0);
    let date_y = date_y_top - date_height / 2.0;
    draw_text(
        &fonts.body,
        rasterizer,
        date_text,
        date_x,
        date_y,
        Palette::TEXT_SECONDARY,
    );

    // Gradient function for smaller cards
    let get_card_gradient = |edge_start_angle: f32| -> (Color, Color) {
        let adjusted_angle = (gradient_angle + edge_start_angle) % 360.0;
        let sector = (adjusted_angle / 90.0) as usize;
        let t_within_sector = (adjusted_angle % 90.0) / 90.0; // 0.0 to 1.0 within sector
        
        let colors = [color_white, color_light, color_mid, color_dark];
        let start_idx = sector % 4;
        let end_idx = (sector + 1) % 4;
        
        // Smoothly interpolate colors at the start and end of this edge
        let start_color = interpolate_color(colors[start_idx], colors[end_idx], t_within_sector);
        let next_sector_idx = (sector + 1) % 4;
        let next_next_idx = (sector + 2) % 4;
        let end_color = interpolate_color(colors[next_sector_idx], colors[next_next_idx], t_within_sector);
        
        (start_color, end_color)
    };

    // Quick actions grid - centered at 70% viewport height
    let quick_actions = [
        ("SLEEP WELL",  "And, Eat well!"),
    ];
    
    for (idx, (label, subtitle)) in quick_actions.iter().enumerate() {
        // Calculate text dimensions first
        let (label_w, label_h) = fonts.body.text_dimensions(label);
        let (sub_w, sub_h) = fonts.small.text_dimensions(subtitle);
        
        // Card dimensions based on content
        let padding_x = px(12.0);
        let padding_y = px(8.0);
        let text_spacing = px(2.0);
        let card_width = label_w.max(sub_w) + padding_x * 2.0;
        let card_height = label_h + text_spacing + sub_h + padding_y * 2.0;
        
        let card_center_y = height_f * 0.70;
        let card_y = card_center_y - card_height / 2.0;
        let card_x = (width_f - card_width) / 2.0;

        // Get gradients for each edge (they flow continuously)
        let (top_start, top_end) = get_card_gradient(0.0);
        let (right_start, right_end) = get_card_gradient(90.0);
        let (bottom_start, bottom_end) = get_card_gradient(180.0);
        let (left_start, left_end) = get_card_gradient(270.0);

        {
            let width = card_width;
            let height = card_height;
            if width > 0.0 && height > 0.0 {
                let mut rect = Rectangle::new()
                    .edge(Edge::Top, StrokeStyle::from_stroke(zeno::Stroke::new(4.0))
                        .horizontal_gradient([(top_start, 0), (top_end, 255)]))
                    .edge(Edge::Right, StrokeStyle::from_stroke(zeno::Stroke::new(4.0))
                        .vertical_gradient([(right_start, 0), (right_end, 255)]))
                    .edge(Edge::Bottom, StrokeStyle::from_stroke(zeno::Stroke::new(4.0))
                        .horizontal_gradient([(bottom_end, 0), (bottom_start, 255)]))
                    .edge(Edge::Left, StrokeStyle::from_stroke(zeno::Stroke::new(4.0))
                        .vertical_gradient([(left_end, 0), (left_start, 255)]))
                    .corner_radii(CornerRadius::new(10., 10.))
                    .bounds(Bounds::new(Point::new(card_x, card_y), Point::new(card_x + width, card_y + height)))
                    .clip(clip);
                let radius = px(6.0);
                if radius > 0.0 {
                    rect = rect.corner_radii(CornerRadius::new(radius, radius));
                }
                rect.draw(rasterizer);
            }
        }

        // Center text both horizontally and vertically in card
        let label_x = card_x + (card_width - label_w) / 2.0;
        let sub_x = card_x + (card_width - sub_w) / 2.0;
        let total_text_height = label_h + text_spacing + sub_h;
        let text_block_start = card_y + (card_height - total_text_height) / 2.0;
        
        draw_text(&fonts.body,  rasterizer, label,    label_x, text_block_start, Palette::BACKGROUND_LIGHT);
        draw_text(&fonts.small, rasterizer, subtitle, sub_x, text_block_start + label_h + text_spacing, Palette::BACKGROUND_LIGHT);
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
    // CSS-style device pixel ratio: actual DPI / baseline DPI
    let device_pixel_ratio = resolution.dpi as f32 / BASELINE_DPI;
    info!(
        "Device pixel ratio: {} (DPI: {} / baseline: {})",
        device_pixel_ratio, resolution.dpi, BASELINE_DPI
    );
    let width_u16  = width  as u16;
    let height_u16 = height as u16;

    let buffer_size = negotiated_pixel_format.framebuffer_size(width as u32, height as u32);
    let mut buffer = alloc::vec![0u8; buffer_size];

    let fonts = UiFonts::new(device_pixel_ratio);

    info!(
        "Compositor UI loop active {:?} @ {}x{}",
        negotiated_pixel_format, width, height
    );

    let mut frame_counter = 0u32;
    let mut snake_angle = 0.0f32; // Snake rotates at constant speed
    let mut gradient_angle = 0.0f32;
    let mut last_frame_time = Instant::now();
    let mut accumulated_time = 0.0f32;
    
    loop {
        let frame_start = Instant::now();
        let time_delta = (frame_start - last_frame_time).as_micros() as f32 / 1_000_000.0;
        last_frame_time = frame_start;
        
        accumulated_time += time_delta;

        // Snake moves at constant speed (slow)
        let snake_speed = 30.0; // degrees per second - slow and smooth
        snake_angle = (snake_angle + snake_speed * time_delta) % 360.0;

        // Calculate speed with Gaussian boost for small cards
        let base_speed = 100.0; // degrees per second
        let boost_peak = 800.0; // peak boost (2x total at peak)
        let cycle_period = 2.0; // 2 second period
        
        let time_in_cycle = accumulated_time % cycle_period;
        
        let gaussian_boost = if time_in_cycle < 1.0 {
            // First second: flat speed (no boost)
            0.0
        } else {
            // Second second: Gaussian boost
            let phase = (time_in_cycle - 1.0) * 2.0 - 1.0; // -1 to 1 over the second half
            boost_peak * (-phase * phase * 4.0).exp()
        };
        
        let current_speed = base_speed + gaussian_boost;
        gradient_angle = (gradient_angle + current_speed * time_delta) % 360.0;

        let render_start = Instant::now();
        render_ui(
            &mut buffer,
            width_u16,
            height_u16,
            frame_counter,
            negotiated_pixel_format,
            device_pixel_ratio,
            &fonts,
            snake_angle,
            gradient_angle,
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
        Timer::after(Duration::from_millis(16)).await;   
    }
}