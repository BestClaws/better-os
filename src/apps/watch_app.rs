/// Fancy Analog Watch Application
///
/// A sophisticated analog watch with luxury aesthetics matching the reference images.
/// Features elegant design with dark background, yellow/gold accents, and white highlights.
/// Pure analog interface - no text rendering required.

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::two_d::{
    Canvas2D, Rasterizer, Point, Size, Rgba8888, Paint, Stroke, Drawable,
    PrimitiveRect as Rect, Circle, Line, Arc, AntiAliasing, FixedI32, U16
};
use crate::libs::gfx::two_d::paint::{LinearGradient, RadialGradient};
use defmt::{info, warn, error};
use embassy_time::{Instant, Duration, Timer};
use micromath::F32Ext;

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

        let frames = self.frame_count.max(1);
        let avg_total = (self.total_draw_us / frames as u64) as u32;
        let avg_update = (self.sum_update_us / frames as u64) as u32;
        let avg_bg = (self.sum_bg_us / frames as u64) as u32;
        let avg_bezel = (self.sum_bezel_us / frames as u64) as u32;
        let avg_minute = (self.sum_minute_track_us / frames as u64) as u32;
        let avg_hour_marks = (self.sum_hour_markers_us / frames as u64) as u32;
        let avg_hands = (self.sum_hands_us / frames as u64) as u32;
        let avg_center = (self.sum_center_jewel_us / frames as u64) as u32;
        let avg_second = (self.sum_second_hand_us / frames as u64) as u32;

        let fps = if avg_total > 0 { 1_000_000 / avg_total } else { 0 };

        // Percentages based on avg_total (guard against div-by-zero)
        let pct = |part: u32| -> u32 { if avg_total > 0 { (part as u64 * 100 / avg_total as u64) as u32 } else { 0 } };

        info!("📊 Watch perf: avg={}μs (fps≈{}), max={}μs", avg_total, fps, self.max_frame_us);
        info!("   update={}μs ({}%), bg={}μs ({}%), bezel={}μs ({}%)",
              avg_update, pct(avg_update), avg_bg, pct(avg_bg), avg_bezel, pct(avg_bezel));
        info!("   minuteTrack={}μs ({}%), hourMarkers={}μs ({}%), hands={}μs ({}%)",
              avg_minute, pct(avg_minute), avg_hour_marks, pct(avg_hour_marks), avg_hands, pct(avg_hands));
        info!("   centerJewel={}μs ({}%), secondHand={}μs ({}%)",
              avg_center, pct(avg_center), avg_second, pct(avg_second));

        // Reset rolling window after reporting to keep numbers fresh
        self.frame_count = 0;
        self.total_draw_us = 0;
        self.max_frame_us = 0;
        self.sum_update_us = 0;
        self.sum_bg_us = 0;
        self.sum_bezel_us = 0;
        self.sum_minute_track_us = 0;
        self.sum_hour_markers_us = 0;
        self.sum_hands_us = 0;
        self.sum_center_jewel_us = 0;
        self.sum_second_hand_us = 0;
        self.last_summary = Instant::now();
    }
}

/// Watch application state
struct WatchState {
    /// Current time components (simulated for demo)
    hours: f32,
    minutes: f32,
    seconds: f32,
    
    /// Animation frame counter for smooth effects
    frame_count: u32,
    
    /// Last update time for smooth animations
    last_update: Instant,
    
    /// Watch face geometry
    center_x: i32,
    center_y: i32,
    radius: f32,
}

impl WatchState {
    fn new(canvas_width: i32, canvas_height: i32) -> Self {
        let center_x = canvas_width / 2;
        let center_y = canvas_height / 2;
        // Dial radius sits comfortably inside a 3px outer ring with a small margin
        let outer_ring_radius = (canvas_width.min(canvas_height) as f32) / 2.0 - 1.5; // 3px ring => 1.5px inset
        let radius = (outer_ring_radius - 8.0).max(0.0);
        
        Self {
            hours: 10.0,      // Classic 10:10 display time
            minutes: 10.0,
            seconds: 30.0,
            frame_count: 0,
            last_update: Instant::now(),
            center_x,
            center_y,
            radius,
        }
    }
    
    /// Update time with smooth animation (real-time progression)
    fn update_time(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_millis() as f32 / 1000.0;
        self.last_update = now;

        // Continuous, smooth progression
        self.seconds += delta;              // seconds advance smoothly
        self.minutes += delta / 60.0;       // minutes advance smoothly
        self.hours += delta / 3600.0;       // hours advance smoothly

        if self.seconds >= 60.0 { self.seconds -= 60.0; }
        if self.minutes >= 60.0 { self.minutes -= 60.0; }
        if self.hours >= 12.0 { self.hours -= 12.0; }
        
        self.frame_count += 1;
    }
}

/// Fancy analog watch application
#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("🕐 Starting Fancy Analog Watch Application");
    
    // Initialize watch state
    let mut watch_state = WatchState::new(0, 0); // Will be updated with actual canvas size
    let mut perf = PerfStats::new();
    
    // Main rendering loop
    loop {
        // Check if the app is focused before drawing
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let draw_start = Instant::now();
        let mut sections: SectionTimes = SectionTimes::default();

        ctx.draw(|surface: &mut DrawingSurface| {
            let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
            let canvas_width = canvas.width() as i32;
            let canvas_height = canvas.height() as i32;
            
            // Update watch state with actual canvas dimensions on first frame
            if watch_state.frame_count == 0 {
                watch_state = WatchState::new(canvas_width, canvas_height);
            }
            
            // Update time and animations
            let t_update = Instant::now();
            watch_state.update_time();
            sections.update_us = t_update.elapsed().as_micros() as u32;
            
            // Render the complete watch interface
            // Background
            let t_bg = Instant::now();
            render_watch_background(&mut canvas, &watch_state);
            sections.bg_us = t_bg.elapsed().as_micros() as u32;

            // Bezel
            let t_bezel = Instant::now();
            render_watch_bezel(&mut canvas, &watch_state);
            sections.bezel_us = t_bezel.elapsed().as_micros() as u32;

            // Minute track
            let t_minute = Instant::now();
            render_minute_track(&mut canvas, &watch_state);
            sections.minute_track_us = t_minute.elapsed().as_micros() as u32;

            // Hour markers
            let t_hour_marks = Instant::now();
            render_hour_markers(&mut canvas, &watch_state);
            sections.hour_markers_us = t_hour_marks.elapsed().as_micros() as u32;

            // Hour and minute hands
            let t_hands = Instant::now();
            render_hour_and_minute_hands(&mut canvas, &watch_state);
            sections.hands_us = t_hands.elapsed().as_micros() as u32;

            // Center jewel
            let t_center = Instant::now();
            render_center_jewel(&mut canvas, &watch_state);
            sections.center_jewel_us = t_center.elapsed().as_micros() as u32;

            // Second hand (on top)
            let t_second = Instant::now();
            render_second_hand(&mut canvas, &watch_state);
            sections.second_hand_us = t_second.elapsed().as_micros() as u32;
            
        }).await;

        let draw_time = draw_start.elapsed();
        let frame_us = draw_time.as_micros() as u32;

        // Record and occasionally summarize
        perf.record_frame(sections, frame_us);
        if frame_us > 20_000 {
            info!("⚠️ Slow frame: {}μs (update={}μs, bg={}μs, bezel={}μs, minute={}μs, hourMarks={}μs, hands={}μs, center={}μs, second={}μs)",
                  frame_us, sections.update_us, sections.bg_us, sections.bezel_us, sections.minute_track_us,
                  sections.hour_markers_us, sections.hands_us, sections.center_jewel_us, sections.second_hand_us);
        }
        perf.maybe_log_summary();
        
        // 60 FPS for smooth animations
        Timer::after(Duration::from_millis(1)).await;
    }
}

/// Render the complete fancy analog watch
fn render_watch_face(canvas: &mut Canvas2D, state: &WatchState) {
    // Retained for potential future single-call render; now measured per-section in draw loop
    render_watch_background(canvas, state);
    render_watch_bezel(canvas, state);
    render_minute_track(canvas, state);
    render_hour_markers(canvas, state);
    render_hour_and_minute_hands(canvas, state);
    render_center_jewel(canvas, state);
    render_second_hand(canvas, state);
}

/// Render background with turquoise to white radial gradient - NO TRANSPARENCY
fn render_watch_background(canvas: &mut Canvas2D, state: &WatchState) {
    let center = Point::new(state.center_x, state.center_y);
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    
    // Ensure entire canvas is painted white first
    Rect::from_coords(0, 0, width as u32, height as u32)
        .fill(Paint::solid(WatchColors::WHITE))
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