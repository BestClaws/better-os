/// Fancy Analog Watch Application
///
/// A sophisticated analog watch with luxury aesthetics matching the reference images.
/// Features elegant design with dark background, yellow/gold accents, and white highlights.
/// Pure analog interface - no text rendering required.

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::two_d::{
    AntiAliasing, Arc, Canvas2D, Circle, Drawable, FixedI32,
    Line, Paint, PrimitiveRect as Rect, Rasterizer, Rgba8888, Stroke, U16
};
use crate::libs::gfx::two_d::paint::{LinearGradient, RadialGradient};
use defmt::{error, info, warn};
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;
use crate::util::math::primitives::{Point, Size};

/// Color palette matching the reference images
struct WatchColors;

impl WatchColors {
    // Core colors with transparency effects for elegance
    const TURQUOISE: Rgba8888 = Rgba8888::new(64, 224, 208, 255);          // Turquoise center
    const WHITE: Rgba8888 = Rgba8888::new(255, 255, 255, 255);             // Pure white
    const PRIMARY_GOLD: Rgba8888 = Rgba8888::new(255, 204, 0, 255);        // Bright yellow/gold
    const ACCENT_GOLD: Rgba8888 = Rgba8888::new(255, 215, 0, 255);         // Gold accent
    const SILVER: Rgba8888 = Rgba8888::new(224, 224, 224, 255);            // Silver/light gray
    const SUBTLE_GRAY: Rgba8888 = Rgba8888::new(68, 68, 68, 255);          // Subtle elements
    const SECOND_HAND_RED: Rgba8888 = Rgba8888::new(255, 68, 68, 255);     // Red accent
    const BLACK: Rgba8888 = Rgba8888::new(0, 0, 0, 255);                   // Pure black
    
    // Transparent overlay colors for sophisticated effects
    const GLASS_HIGHLIGHT: Rgba8888 = Rgba8888::new(255, 255, 255, 80);    // Glass reflection
    const SHADOW: Rgba8888 = Rgba8888::new(0, 0, 0, 60);                   // Subtle shadows
    const GLOW: Rgba8888 = Rgba8888::new(255, 204, 0, 120);                // Gold glow
}

/// Per-frame section timing (in microseconds)
#[derive(Clone, Copy, Default)]
struct SectionTimes {
    update_us: u32,
    bg_us: u32,
    bezel_us: u32,
    minute_track_us: u32,
    hour_markers_us: u32,
    hands_us: u32,
    center_jewel_us: u32,
    second_hand_us: u32,
}

/// Aggregated performance statistics
struct PerfStats {
    frame_count: u32,
    total_draw_us: u64,
    max_frame_us: u32,

    sum_update_us: u64,
    sum_bg_us: u64,
    sum_bezel_us: u64,
    sum_minute_track_us: u64,
    sum_hour_markers_us: u64,
    sum_hands_us: u64,
    sum_center_jewel_us: u64,
    sum_second_hand_us: u64,
    last_summary: Instant,
}

impl PerfStats {
    fn new() -> Self {
        Self {
            frame_count: 0,
            total_draw_us: 0,
            max_frame_us: 0,
            sum_update_us: 0,
            sum_bg_us: 0,
            sum_bezel_us: 0,
            sum_minute_track_us: 0,
            sum_hour_markers_us: 0,
            sum_hands_us: 0,
            sum_center_jewel_us: 0,
            sum_second_hand_us: 0,
            last_summary: Instant::now(),
        }
    }

    fn record_frame(&mut self, sections: SectionTimes, total_draw_us: u32) {
        self.frame_count = self.frame_count.wrapping_add(1);
        self.total_draw_us += total_draw_us as u64;
        if total_draw_us > self.max_frame_us { self.max_frame_us = total_draw_us; }

        self.sum_update_us += sections.update_us as u64;
        self.sum_bg_us += sections.bg_us as u64;
        self.sum_bezel_us += sections.bezel_us as u64;
        self.sum_minute_track_us += sections.minute_track_us as u64;
        self.sum_hour_markers_us += sections.hour_markers_us as u64;
        self.sum_hands_us += sections.hands_us as u64;
        self.sum_center_jewel_us += sections.center_jewel_us as u64;
        self.sum_second_hand_us += sections.second_hand_us as u64;
    }

    fn maybe_log_summary(&mut self) {
        // Log every ~60 frames or every 5 seconds, whichever comes first
        if self.frame_count % 60 != 0 && self.last_summary.elapsed().as_secs() < 5 { return; }
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Rasterizer, Circle, line::Line};
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use crate::util::math::primitives::Point;

/// Simplified Analog Watch Application (migrated to new gfx API)
#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting simplified watch app");
    loop {
        if !ctx.is_focused().await { Timer::after(Duration::from_millis(100)).await; continue; }
        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let raster: &mut dyn Rasterizer = surface;
            let w = raster.width() as i32;
            let h = raster.height() as i32;
            raster.fill_rect(0, 0, w, h, Rgba8888::rgba(20, 25, 35, 255));

            // Bezel
            Circle::new(Point::new(w/2, h/2), (h.min(w) / 2 - 4))
                .stroke(2, Rgba8888::rgba(200, 200, 200, 255))
                .draw(raster);

            // Hands (static)
            Line::new(Point::new(w/2, h/2), Point::new(w/2, h/4))
                .stroke(4, Rgba8888::rgba(255, 215, 0, 255))
                .draw(raster);
            Line::new(Point::new(w/2, h/2), Point::new(w*3/4, h/2))
                .stroke(3, Rgba8888::rgba(255, 255, 255, 255))
                .draw(raster);
        }).await;
        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(16)).await;
    }
}
        .aa(AntiAliasing::High)
        .draw(canvas);

    // Quick radial fade overlay: turquoise core transitions rapidly to white
    let fade_radius = (width.min(height) as f32) * 0.36;
    Circle::new(center, fade_radius)
        .fill(Paint::radial(
            center,
            fade_radius,
            WatchColors::TURQUOISE,  // Turquoise center
            WatchColors::WHITE       // White edges
        ))
        .aa(AntiAliasing::High)
        .draw(canvas);
}

/// Render sophisticated bezel with metallic appearance
fn render_watch_bezel(canvas: &mut Canvas2D, state: &WatchState) {
    let center = Point::new(state.center_x, state.center_y);
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;

    // Outermost ring: subtle gold, flush with canvas edge (no padding)
    let ring_radius = (width.min(height) as f32) / 2.0 - 1.0; // stroke is centered
    Circle::new(center, ring_radius)
        .stroke(Stroke::new(WatchColors::ACCENT_GOLD, 2.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
}

/// Render minute track with fine markings
fn render_minute_track(canvas: &mut Canvas2D, state: &WatchState) {
    let center = Point::new(state.center_x, state.center_y);
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let ring_radius = (width.min(height) as f32) / 2.0 - 1.0;
    
    // Draw minute markers (60 total)
    for minute in 0..60 {
        let angle = (minute as f32 * 6.0 - 90.0) * core::f32::consts::PI / 180.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        let is_five_minute = minute % 5 == 0;
        let is_quarter_hour = minute % 15 == 0;
        
        let (outer_radius, inner_radius, color, width_px) = if is_quarter_hour {
            // Quarter hour marks - near ring, thin, gold
            (ring_radius - 3.0, ring_radius - 13.0, WatchColors::PRIMARY_GOLD, 1.5)
        } else if is_five_minute {
            // Five minute marks - near ring, thinner, white
            (ring_radius - 3.0, ring_radius - 10.0, WatchColors::WHITE, 1.2)
        } else {
            // Single minute marks - very short, subtle
            (ring_radius - 3.0, ring_radius - 7.0, WatchColors::SUBTLE_GRAY, 0.8)
        };
        
        let outer_x = state.center_x + (outer_radius * cos_a) as i32;
        let outer_y = state.center_y + (outer_radius * sin_a) as i32;
        let inner_x = state.center_x + (inner_radius * cos_a) as i32;
        let inner_y = state.center_y + (inner_radius * sin_a) as i32;
        
        Line::new(
            Point::new(inner_x, inner_y),
            Point::new(outer_x, outer_y)
        )
        .stroke(Stroke::new(color, width_px))
        .aa(AntiAliasing::Medium)
        .draw(canvas);
    }
}

/// Render elegant hour markers with distinctive shapes
fn render_hour_markers(canvas: &mut Canvas2D, state: &WatchState) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let ring_radius = (width.min(height) as f32) / 2.0 - 1.0;

    for hour in 0..12 {
        let angle = (hour as f32 * 30.0 - 90.0) * core::f32::consts::PI / 180.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        match hour {
            0 => {
                // 12 o'clock - Large diamond shape (gold)
                let marker_distance = state.radius - 15.0;
                let marker_x = state.center_x + (marker_distance * cos_a) as i32;
                let marker_y = state.center_y + (marker_distance * sin_a) as i32;
                render_diamond_marker(canvas, marker_x, marker_y, 8.0, WatchColors::PRIMARY_GOLD);
            },
            3 | 6 | 9 => {
                // 3, 6, 9 o'clock - Shorter rectangles (white)
                let marker_distance = state.radius - 12.0;
                let marker_x = state.center_x + (marker_distance * cos_a) as i32;
                let marker_y = state.center_y + (marker_distance * sin_a) as i32;
                render_rectangle_marker(canvas, marker_x, marker_y, 4.0, 8.0, WatchColors::WHITE);
            },
            _ => {
                // Subtle hour index very close to outer ring for readability
                let outer = ring_radius - 6.0;
                let inner = ring_radius - 12.0;
                let ox = state.center_x + (outer * cos_a) as i32;
                let oy = state.center_y + (outer * sin_a) as i32;
                let ix = state.center_x + (inner * cos_a) as i32;
                let iy = state.center_y + (inner * sin_a) as i32;
                Line::new(Point::new(ix, iy), Point::new(ox, oy))
                    .stroke(Stroke::new(WatchColors::SILVER, 1.0))
                    .aa(AntiAliasing::High)
                    .draw(canvas);
            }
        }
    }
}

/// Render diamond-shaped marker
fn render_diamond_marker(canvas: &mut Canvas2D, x: i32, y: i32, size: f32, color: Rgba8888) {
    let half_size = size / 2.0;
    
    // Draw diamond as four triangular lines
    Line::new(Point::new(x, y - half_size as i32), Point::new(x + half_size as i32, y))
        .stroke(Stroke::new(color, 3.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    Line::new(Point::new(x + half_size as i32, y), Point::new(x, y + half_size as i32))
        .stroke(Stroke::new(color, 3.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    Line::new(Point::new(x, y + half_size as i32), Point::new(x - half_size as i32, y))
        .stroke(Stroke::new(color, 3.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    Line::new(Point::new(x - half_size as i32, y), Point::new(x, y - half_size as i32))
        .stroke(Stroke::new(color, 3.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Fill the diamond
    Circle::new(Point::new(x, y), size * 0.6)
        .fill(Paint::solid(color))
        .aa(AntiAliasing::High)
        .draw(canvas);
}

/// Render rectangular marker
fn render_rectangle_marker(canvas: &mut Canvas2D, x: i32, y: i32, width: f32, height: f32, color: Rgba8888) {
    Rect::from_coords(
        x - (width / 2.0) as i32,
        y - (height / 2.0) as i32,
        width as u32,
        height as u32
    )
    .fill(Paint::solid(color))
    .corner_radius(2.0)
    .aa(AntiAliasing::High)
    .draw(canvas);
}

/// Render hour and minute hands with shadow effects
fn render_hour_and_minute_hands(canvas: &mut Canvas2D, state: &WatchState) {
    let center = Point::new(state.center_x, state.center_y);
    let shadow_offset = 2;
    
    // Hour hand - with shadow for depth (use continuous hours + minutes contribution)
    let hour_angle = ((state.hours % 12.0) * 30.0 + state.minutes * 0.5 - 90.0) * core::f32::consts::PI / 180.0;
    let hour_length = state.radius * 0.5;
    let hour_end_x = state.center_x + (hour_length * hour_angle.cos()) as i32;
    let hour_end_y = state.center_y + (hour_length * hour_angle.sin()) as i32;
    
    // Hour hand shadow
    Line::new(
        Point::new(center.x + shadow_offset, center.y + shadow_offset),
        Point::new(hour_end_x + shadow_offset, hour_end_y + shadow_offset)
    )
    .stroke(Stroke::new(WatchColors::SHADOW, 4.0))
    .aa(AntiAliasing::High)
    .draw(canvas);
    
    // Hour hand main body
    Line::new(center, Point::new(hour_end_x, hour_end_y))
        .stroke(Stroke::new(WatchColors::WHITE, 4.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Hour hand gold accent
    Line::new(center, Point::new(hour_end_x, hour_end_y))
        .stroke(Stroke::new(WatchColors::ACCENT_GOLD, 1.5))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Minute hand - with shadow and glow (continuous)
    let minute_angle = (state.minutes * 6.0 - 90.0) * core::f32::consts::PI / 180.0;
    let minute_length = state.radius * 0.75;
    let minute_end_x = state.center_x + (minute_length * minute_angle.cos()) as i32;
    let minute_end_y = state.center_y + (minute_length * minute_angle.sin()) as i32;
    
    // Minute hand shadow
    Line::new(
        Point::new(center.x + shadow_offset, center.y + shadow_offset),
        Point::new(minute_end_x + shadow_offset, minute_end_y + shadow_offset)
    )
    .stroke(Stroke::new(WatchColors::SHADOW, 3.0))
    .aa(AntiAliasing::High)
    .draw(canvas);
    
    // Minute hand glow effect
    Line::new(center, Point::new(minute_end_x, minute_end_y))
        .stroke(Stroke::new(WatchColors::GLOW, 5.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Minute hand main body
    Line::new(center, Point::new(minute_end_x, minute_end_y))
        .stroke(Stroke::new(WatchColors::PRIMARY_GOLD, 3.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
}

/// Render second hand on top of center jewel
fn render_second_hand(canvas: &mut Canvas2D, state: &WatchState) {
    // Second hand - thin red with counterbalance (drawn on top)
    let second_angle = (state.seconds * 6.0 - 90.0) * core::f32::consts::PI / 180.0;
    let second_length = state.radius * 0.85;
    let second_tail_length = state.radius * 0.25;
    
    let second_end_x = state.center_x + (second_length * second_angle.cos()) as i32;
    let second_end_y = state.center_y + (second_length * second_angle.sin()) as i32;
    let second_tail_x = state.center_x - (second_tail_length * second_angle.cos()) as i32;
    let second_tail_y = state.center_y - (second_tail_length * second_angle.sin()) as i32;
    
    // Second hand main line
    Line::new(Point::new(second_tail_x, second_tail_y), Point::new(second_end_x, second_end_y))
        .stroke(Stroke::new(WatchColors::SECOND_HAND_RED, 2.0))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Second hand tip accent
    Circle::new(Point::new(second_end_x, second_end_y), 2.0)
        .fill(Paint::solid(WatchColors::SECOND_HAND_RED))
        .aa(AntiAliasing::High)
        .draw(canvas);

    // Center hub for seconds hand
    Circle::new(Point::new(state.center_x, state.center_y), 3.0)
        .fill(Paint::solid(WatchColors::SECOND_HAND_RED))
        .aa(AntiAliasing::High)
        .draw(canvas);
}

/// Render elegant center jewel with layered design
fn render_center_jewel(canvas: &mut Canvas2D, state: &WatchState) {
    let center = Point::new(state.center_x, state.center_y);
    
    // Smaller center hub - outer ring gold
    Circle::new(center, 6.0)
        .fill(Paint::solid(WatchColors::PRIMARY_GOLD))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Middle ring - white/silver
    Circle::new(center, 4.0)
        .fill(Paint::solid(WatchColors::WHITE))
        .aa(AntiAliasing::High)
        .draw(canvas);
    
    // Center dot - black for contrast
    Circle::new(center, 2.0)
        .fill(Paint::solid(WatchColors::BLACK))
        .aa(AntiAliasing::High)
        .draw(canvas);
}