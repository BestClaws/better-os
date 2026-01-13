//! Minimalist watch face optimized for rectangular displays.

use rust_gfx::color::Rgba8888;
use rust_gfx::rasterizer::Rasterizer;
use rust_gfx::primitives::{RectDsc, draw_rect, arc::{ArcDsc, draw_arc}};
use rust_gfx::types::{Point, Area, Gradient, OPA_COVER};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;

#[embassy_executor::task]
pub async fn watch_app(ctx: AppContext) {
    info!("Starting watch app");
    let start = Instant::now();
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let width = surface.width() as i32;
            let height = surface.height() as i32;

            let elapsed = Instant::now() - start;
            let secs_f = elapsed.as_micros() as f32 / 1_000_000.0;

            let seconds = secs_f % 60.0;
            let minutes = (secs_f / 60.0) % 60.0;
            let hours = (secs_f / 3600.0) % 24.0;

            draw_background(surface, width, height);
            draw_time_arcs(surface, width, height, hours, minutes, seconds);
        })
        .await;

        let t = draw_start.elapsed();
        info!("watch frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(1000)).await;
    }
}

fn draw_background(surface: &mut DrawingSurface, width: i32, height: i32) {
    // Deep gradient background
    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(8, 8, 12, 255);
    bg.bg_opa = OPA_COVER;
    bg.bg_grad = Gradient::vertical(
        Rgba8888::rgba(12, 12, 18, 255),
        Rgba8888::rgba(4, 4, 8, 255)
    );
    let bg_area = Area::new(0, 0, width, height);
    draw_rect(surface, &bg, &bg_area);
}

fn draw_time_arcs(
    surface: &mut DrawingSurface,
    width: i32,
    height: i32,
    hours: f32,
    minutes: f32,
    seconds: f32,
) {
    let cx = width / 2;
    let cy = height / 2;
    
    // Calculate base radius from smaller dimension
    let base_radius = (width.min(height) as f32 * 0.35) as i32;
    
    // Arc parameters - concentric rings
    let arc_width = (base_radius as f32 * 0.12) as i32;
    let arc_gap = (base_radius as f32 * 0.08) as i32;
    
    // Hours arc (outermost)
    let hour_radius = base_radius;
    let hour_angle = (hours / 24.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        hour_radius,
        arc_width,
        hour_angle,
        Rgba8888::rgba(100, 140, 255, 255),
        Rgba8888::rgba(40, 60, 120, 100),
    );
    
    // Minutes arc (middle)
    let minute_radius = base_radius - arc_width - arc_gap;
    let minute_angle = (minutes / 60.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        minute_radius,
        arc_width,
        minute_angle,
        Rgba8888::rgba(120, 255, 180, 255),
        Rgba8888::rgba(40, 100, 60, 100),
    );
    
    // Seconds arc (innermost)
    let second_radius = minute_radius - arc_width - arc_gap;
    let second_angle = (seconds / 60.0) * 360.0;
    draw_arc_ring(
        surface,
        cx,
        cy,
        second_radius,
        arc_width,
        second_angle,
        Rgba8888::rgba(255, 100, 120, 255),
        Rgba8888::rgba(120, 40, 60, 100),
    );
    
    // Center info
    draw_center_info(surface, cx, cy, second_radius - arc_width - arc_gap, hours, minutes);
}

fn draw_arc_ring(
    surface: &mut DrawingSurface,
    cx: i32,
    cy: i32,
    radius: i32,
    width: i32,
    angle: f32,
    active_color: Rgba8888,
    track_color: Rgba8888,
) {
    // Background track (full circle)
    let mut track = ArcDsc::new(Point::new(cx, cy), radius, 0, 360);
    track.width = width;
    track.color = track_color;
    track.opa = OPA_COVER;
    draw_arc(surface, &track);
    
    // Active arc (progress)
    if angle > 0.1 {
        let mut active = ArcDsc::new(Point::new(cx, cy), radius, -90, angle as i32 - 90);
        active.width = width;
        active.color = active_color;
        active.opa = OPA_COVER;
        active.rounded = true;
        draw_arc(surface, &active);
    }
}

fn draw_center_info(
    surface: &mut DrawingSurface,
    cx: i32,
    cy: i32,
    max_radius: i32,
    hours: f32,
    minutes: f32,
) {
    // Subtle center circle with time info
    let info_radius = (max_radius as f32 * 0.7) as i32;
    
    // Background circle
    let mut bg = RectDsc::new();
    bg.bg_color = Rgba8888::rgba(16, 16, 24, 200);
    bg.bg_opa = OPA_COVER;
    bg.bg_grad = Gradient::radial(
        Rgba8888::rgba(20, 20, 28, 200),
        Rgba8888::rgba(12, 12, 16, 200)
    );
    bg.radius = 32767; // RADIUS_CIRCLE
    bg.border_width = 1;
    bg.border_color = Rgba8888::rgba(60, 60, 80, 150);
    bg.border_opa = OPA_COVER;
    let bg_area = Area::new(
        cx - info_radius,
        cy - info_radius,
        info_radius * 2,
        info_radius * 2
    );
    draw_rect(surface, &bg, &bg_area);
    
    // Small dots to indicate time positions
    let hours_12 = (hours % 12.0) as i32;
    let minutes_u = minutes as i32;
    
    // Hour dots
    let dot_radius = (info_radius as f32 * 0.06) as i32;
    for h in 0..12 {
        let angle = (h as f32 * 30.0 - 90.0).to_radians();
        let dot_dist = (info_radius as f32 * 0.7) as i32;
        let dot_x = cx + (dot_dist as f32 * angle.cos()) as i32;
        let dot_y = cy + (dot_dist as f32 * angle.sin()) as i32;
        
        let is_current = h == hours_12;
        let color = if is_current {
            Rgba8888::rgba(100, 140, 255, 255)
        } else {
            Rgba8888::rgba(40, 40, 60, 100)
        };
        
        let mut dot = RectDsc::new();
        dot.bg_color = color;
        dot.bg_opa = OPA_COVER;
        dot.radius = 32767;
        let dot_size = if is_current { dot_radius } else { dot_radius / 2 };
        let dot_area = Area::new(
            dot_x - dot_size,
            dot_y - dot_size,
            dot_size * 2,
            dot_size * 2
        );
        draw_rect(surface, &dot, &dot_area);
    }
}
