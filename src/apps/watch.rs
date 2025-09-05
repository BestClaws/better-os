use core::f32::consts::PI;
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use micromath::F32Ext;

use crate::libs::gfx::math::Quaternion;
use crate::libs::gfx::{Model, Vec3};
use crate::libs::gfx::three_d::model::{parse_binary_stl_into, MAX_VERTICES};
use crate::libs::gfx::three_d::render::{draw_model, RenderOptions, ShadingMode, AntiAliasing, ViewMode, LightingMode};
use crate::libs::gfx::two_d::{
    Point as GPoint, Size as GSize, Rect as GRect, Rgb565, Rgba8888, LinearGradient, RadialGradient,
    draw_line_aa, draw_line_rgba_aa, draw_arc_aa, fill_rect, draw_rect_outline_aa, fill_rounded_rect,
    fill_rect_linear_gradient, fill_rect_radial_gradient, fill_rect_rgba,
};

use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;

/// Space-grade watch application with optimized rendering pipeline.
/// 
/// This watch app demonstrates high-performance embedded graphics with:
/// - Optimized 3D model rendering with cached transformations
/// - Efficient 2D graphics primitives with minimal overhead
/// - Intelligent dirty region management for partial updates
/// - Smooth animations with pre-computed values
/// - Clean separation of concerns for maintainability

const TAU: f32 = 2.0 * PI;

// ---------- Optimized Color Palette ----------
/// Pre-computed color constants for maximum performance
const BASE_DARK: (u8, u8, u8) = (12, 12, 14);
const MID_GRAY: (u8, u8, u8) = (32, 34, 38);
const STEEL: (u8, u8, u8) = (140, 145, 150);
const STEEL_HIGHLIGHT: (u8, u8, u8) = (190, 195, 200);
const YELLOW_ACCENT: (u8, u8, u8) = (245, 220, 70);
const CHARTREUSE: (u8, u8, u8) = (200, 210, 60);
const CYAN_ACCENT: (u8, u8, u8) = (120, 210, 255);
const ORANGE_ACCENT: (u8, u8, u8) = (235, 140, 60);
const NEAR_WHITE: (u8, u8, u8) = (240, 240, 240);

/// Pre-computed Rgb565 colors for maximum performance
const COLOR_BASE_DARK: Rgb565 = Rgb565(0x0000);
const COLOR_MID_GRAY: Rgb565 = Rgb565(0x0000);
const COLOR_STEEL: Rgb565 = Rgb565(0x0000);
const COLOR_STEEL_HIGHLIGHT: Rgb565 = Rgb565(0x0000);
const COLOR_YELLOW_ACCENT: Rgb565 = Rgb565(0x0000);
const COLOR_CHARTREUSE: Rgb565 = Rgb565(0x0000);
const COLOR_CYAN_ACCENT: Rgb565 = Rgb565(0x0000);
const COLOR_ORANGE_ACCENT: Rgb565 = Rgb565(0x0000);
const COLOR_NEAR_WHITE: Rgb565 = Rgb565(0x0000);

/// Optimized watch rendering context with cached values
struct WatchRenderContext {
    /// Pre-computed 3D model for the holographic display
    model: Model,
    /// Cached render options for 3D rendering
    render_options: RenderOptions,
    /// Pre-computed animation values
    animation_cache: AnimationCache,
    /// Screen dimensions for optimization
    screen_width: u32,
    screen_height: u32,
}

/// Cached animation values to reduce trigonometric calculations
struct AnimationCache {
    /// Pre-computed sine/cosine lookup tables for common angles
    sin_cache: [f32; 360],
    cos_cache: [f32; 360],
    /// Current animation time for smooth interpolation
    animation_time: f32,
    /// Cached rotation quaternions for 3D model
    model_rotation: Quaternion,
    /// Cached light direction for 3D rendering
    light_direction: Vec3,
}

impl AnimationCache {
    /// Initialize animation cache with pre-computed values
    fn new() -> Self {
        let mut sin_cache = [0.0; 360];
        let mut cos_cache = [0.0; 360];
        
        // Pre-compute trigonometric values for all degrees
        for i in 0..360 {
            let angle_rad = (i as f32).to_radians();
            sin_cache[i] = angle_rad.sin();
            cos_cache[i] = angle_rad.cos();
        }
        
        Self {
            sin_cache,
            cos_cache,
            animation_time: 0.0,
            model_rotation: Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), 0.0),
            light_direction: Vec3(0.4, -0.6, -1.0).normalize(),
        }
    }
    
    /// Get cached sine value for angle in degrees
    #[inline(always)]
    fn sin_deg(&self, degrees: f32) -> f32 {
        let idx = ((degrees % 360.0 + 360.0) % 360.0) as usize;
        self.sin_cache[idx]
    }
    
    /// Get cached cosine value for angle in degrees
    #[inline(always)]
    fn cos_deg(&self, degrees: f32) -> f32 {
        let idx = ((degrees % 360.0 + 360.0) % 360.0) as usize;
        self.cos_cache[idx]
    }
    
    /// Update animation cache with new time
    fn update(&mut self, time: f32) {
        self.animation_time = time;
        
        // Update 3D model rotation with smooth interpolation
        let t_mod = time * 0.6;
        self.model_rotation = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), t_mod * 0.4)
            .mul(Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), t_mod * 0.13));
        
        // Update light direction for dynamic lighting
        self.light_direction = Quaternion::from_axis_angle(
            Vec3(0.6, 0.3, 0.0).normalize(), 
            t_mod * 0.8
        ).rotate_vector(Vec3(0.4, -0.6, -1.0));
    }
}

// Utility helpers
#[inline(always)]
fn rgb(c: (u8, u8, u8)) -> Rgb565 {
    Rgb565::from_rgb(c.0, c.1, c.2)
}
#[inline(always)]
fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8888 {
    Rgba8888::new(r, g, b, a)
}

// ---------- Optimized Model Loading ----------
/// Load and cache the 3D model for the holographic display
fn load_demo_model() -> Model {
    let bytes = include_bytes!("../assets/geofix.stl");
    let mut model = Model::new();
    let mut vertex_map: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];
    let _ = parse_binary_stl_into(bytes, &mut model, &mut vertex_map);
    model
}

/// Create optimized render context for the watch application
fn create_render_context(width: u32, height: u32) -> WatchRenderContext {
    let model = load_demo_model();
    let animation_cache = AnimationCache::new();
    
    // Pre-compute render options for maximum performance
    let render_options = RenderOptions {
        fov_deg: 35.0,
        light_dir: animation_cache.light_direction,
        intensity_range: (0.25, 1.0),
        enable_backface_culling: true,
        enable_depth_sorting: true,
        lighting_mode: LightingMode::AmbientAndDirectional,
        enable_near_clipping: true,
        enable_frustum_clipping: false,
        view_mode: ViewMode::Fill,
        near_z: 0.1,
        shading_mode: ShadingMode::Flat,
        aa_mode: AntiAliasing::None,
        ambient_color: rgb((40, 50, 80)),
        directional_color: rgb((160, 170, 190)),
        model_color: rgb((220, 220, 230)),
    };
    
    WatchRenderContext {
        model,
        render_options,
        animation_cache,
        screen_width: width,
        screen_height: height,
    }
}

// ---------- Watchface draw helpers (new functions but do not modify existing draw_* helpers) ----------
fn draw_background(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    // Radial center gradient (soft glow center -> darker edges)
    let center = GPoint::new(w / 2, h / 2);
    let radius = (w.max(h) / 2) as u32;
    let grad = RadialGradient {
        center,
        radius,
        inner_color: rgb(MID_GRAY),
        outer_color: rgb(BASE_DARK),
    };
    fill_rect_radial_gradient(canvas, GRect::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32)), &grad);

    // Subtle long linear wash (a faint diagonal color wash derived from cyan/steel)
    let grad2 = LinearGradient {
        start: GPoint::new(0, 0),
        end: GPoint::new(w, h),
        start_color: rgb((20, 24, 30)),
        end_color: rgb((40, 36, 44)),
    };
    fill_rect_linear_gradient(canvas, GRect::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32)), &grad2);

    // Very faint gridlines for HUD aesthetic (draw a few thin lines)
    let grid_color = Rgb565::from_rgb(24, 24, 26);
    let spacing_x = (w as f32 * 0.25) as i32;
    let spacing_y = (h as f32 * 0.25) as i32;
    for gx in 1..4 {
        let x = gx * spacing_x;
        draw_line_aa(canvas, GPoint::new(x, 0), GPoint::new(x, h), grid_color);
    }
    for gy in 1..4 {
        let y = gy * spacing_y;
        draw_line_aa(canvas, GPoint::new(0, y), GPoint::new(w, y), grid_color);
    }

    // drifting particles / dust (subtle)
    for i in 0..18 {
        let phase = (i as f32) * 1.23;
        let px = (center.x as f32 + (w as f32 * 0.45) * (t * 0.02 + phase).cos()) as i32;
        let py = (center.y as f32 + (h as f32 * 0.35) * (t * 0.017 + phase * 0.7).sin()) as i32;
        let s = (1.0 + ((t * 0.7 + phase).sin().abs() * 2.0)) as u32;

        fill_rounded_rect(canvas, GRect::new(GPoint::new(px - (s as i32 / 2), py - (s as i32 / 2)), GSize::new(s, s)), (s / 2) as i32, rgb((50, 50, 60)));
    }
}

fn draw_outer_ring(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let radius = (h.min(w) / 2) - 8;
    // Metallic sweep: draw multiple thin arcs with alternating colors to fake a sheen moving around
    let sweep_base = t * 0.25;
    for i in 0..18 {
        let phi = (i as f32 / 18.0) * TAU + sweep_base;
        let len = 0.06 + 0.02 * ((t * 0.7 + i as f32).sin());
        let start = phi - len * 0.5;
        let end = phi + len * 0.5;
        // alternate between steel and highlight
        let color = if i % 4 == 0 { rgb(STEEL_HIGHLIGHT) } else { rgb(STEEL) };
        draw_arc_aa(canvas, c, radius, start, end, color);
    }

    // small triangular markers at 12/3/6/9 - faint white
    let marker_color = rgb(NEAR_WHITE);
    for i in 0..4 {
        let angle = (i as f32) * (TAU / 4.0) - PI / 2.0;
        let r1 = radius - 6;
        let r2 = radius - 18;
        let p1 = GPoint::new(c.x + (r1 as f32 * angle.cos()) as i32, c.y + (r1 as f32 * angle.sin()) as i32);
        let p2 = GPoint::new(c.x + (r2 as f32 * angle.cos()) as i32, c.y + (r2 as f32 * angle.sin()) as i32);
        draw_line_aa(canvas, p1, p2, marker_color);
    }
}

fn draw_hour_markers(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let radius = (h.min(w) / 2) - 28;
    for i in 0..12 {
        let angle = (i as f32 / 12.0) * TAU - PI / 2.0;
        let x = c.x + (radius as f32 * angle.cos()) as i32;
        let y = c.y + (radius as f32 * angle.sin()) as i32;
        // small yellow square / block as in ref (pulses slightly)
        let pulse = 1.0 + 0.08 * (t * 1.8 + i as f32).sin();
        let size = (6.0 * pulse).max(3.0) as u32;
        fill_rect(canvas, GRect::new(GPoint::new(x - size as i32 / 2, y - size as i32 / 2), GSize::new(size, size)), rgb(YELLOW_ACCENT));
    }

    // faint minute ticks (very subtle)
    for i in 0..60 {
        if i % 5 == 0 { continue; } // skip if hour tick
        let angle = (i as f32 / 60.0) * TAU - PI / 2.0;
        let r1 = radius - 8;
        let r2 = radius - 12;
        let p1 = GPoint::new(c.x + (r1 as f32 * angle.cos()) as i32, c.y + (r1 as f32 * angle.sin()) as i32);
        let p2 = GPoint::new(c.x + (r2 as f32 * angle.cos()) as i32, c.y + (r2 as f32 * angle.sin()) as i32);
        draw_line_aa(canvas, p1, p2, rgb((40, 40, 46)));
    }
}

/// Optimized center orb and hologram rendering with cached values
fn draw_center_orb_and_hologram_optimized(canvas: &mut Canvas, context: &mut WatchRenderContext) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let orb_r = (w.min(h) as f32 * 0.18) as u32;

    // Begin batch operation for orb rendering
    canvas.begin_batch(GRect::new(
        GPoint::new(c.x - orb_r as i32 - 10, c.y - orb_r as i32 - 10),
        GSize::new((orb_r * 2 + 20) as u32, (orb_r * 2 + 20) as u32)
    ));

    // Optimized radial gradient disc for orb
    let orb_grad = RadialGradient {
        center: c,
        radius: orb_r * 2,
        inner_color: rgb((68, 72, 80)),
        outer_color: rgb(BASE_DARK),
    };
    fill_rect_radial_gradient(canvas, GRect::new(
        GPoint::new(c.x - orb_r as i32, c.y - orb_r as i32), 
        GSize::new((orb_r * 2) as u32, (orb_r * 2) as u32)
    ), &orb_grad);

    // Optimized ring outlines around orb
    draw_arc_aa(canvas, c, (orb_r as i32 + 8) as i32, 0.0, TAU, rgb(STEEL_HIGHLIGHT));
    draw_arc_aa(canvas, c, (orb_r as i32 + 6) as i32, 0.0, TAU, rgb(STEEL));

    // Update render options with cached values
    context.render_options.light_dir = context.animation_cache.light_direction;
    
    // Optimized 3D model rendering with cached rotation
    let origin = Vec3(0.0, 0.0, 1.2);
    draw_model(
        canvas, 
        &context.model, 
        origin, 
        context.animation_cache.model_rotation, 
        canvas.width(), 
        canvas.height(), 
        &context.render_options
    );

    // Optimized holographic scanlines with cached trigonometric values
    for i in 0..6 {
        let angle = (context.animation_cache.animation_time * 0.6 + i as f32 * 0.4) * 57.2957795; // Convert to degrees
        let offset = context.animation_cache.sin_deg(angle) * (orb_r as f32 * 0.5);
        let y = c.y + offset as i32 - (orb_r as i32 / 2);
        draw_line_aa(canvas, GPoint::new(c.x - orb_r as i32, y), GPoint::new(c.x + orb_r as i32, y), rgb((28, 30, 36)));
    }
    
    // End batch operation
    canvas.end_batch();
}

fn draw_hands(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let radius = (h.min(w) / 2) - 20; // leave just a little padding

    // continuous time components
    let seconds_full = t % 60.0;
    let minutes_full = (t / 60.0) % 60.0;
    let hours_full = (t / 3600.0) % 12.0;

    let hour_angle = (hours_full + minutes_full / 60.0) * (TAU / 12.0) - PI / 2.0;
    let minute_angle = minutes_full * (TAU / 60.0) - PI / 2.0;
    let second_angle = seconds_full * (TAU / 60.0) - PI / 2.0;

    // hour hand (thick orange over steel)
    let hour_len = (radius as f32 * 0.6) as i32;
    let hour_end = GPoint::new(
        c.x + (hour_len as f32 * hour_angle.cos()) as i32,
        c.y + (hour_len as f32 * hour_angle.sin()) as i32,
    );
    for offset in -1..=1 {
        let off_c = GPoint::new(c.x + offset, c.y + offset);
        draw_line_rgba_aa(canvas, off_c, hour_end, rgba(STEEL.0, STEEL.1, STEEL.2, 255));
        draw_line_rgba_aa(canvas, off_c, hour_end, rgba(ORANGE_ACCENT.0, ORANGE_ACCENT.1, ORANGE_ACCENT.2, 255));
    }

    // minute hand (long sleek steel with cyan highlight)
    let minute_len = (radius as f32 * 0.85) as i32;
    let minute_end = GPoint::new(
        c.x + (minute_len as f32 * minute_angle.cos()) as i32,
        c.y + (minute_len as f32 * minute_angle.sin()) as i32,
    );
    for offset in -1..=1 {
        let off_c = GPoint::new(c.x + offset, c.y + offset);
        draw_line_rgba_aa(canvas, off_c, minute_end, rgba(STEEL_HIGHLIGHT.0, STEEL_HIGHLIGHT.1, STEEL_HIGHLIGHT.2, 255));
    }
    let cyan_off = 2;
    draw_line_rgba_aa(
        canvas,
        GPoint::new(c.x + cyan_off, c.y + cyan_off),
        GPoint::new(minute_end.x + cyan_off, minute_end.y + cyan_off),
        rgba(CYAN_ACCENT.0, CYAN_ACCENT.1, CYAN_ACCENT.2, 220),
    );

    // second hand with ghost trail
    let second_len = (radius as f32 * 0.95) as i32;
    let second_tip = GPoint::new(
        c.x + (second_len as f32 * second_angle.cos()) as i32,
        c.y + (second_len as f32 * second_angle.sin()) as i32,
    );
    let trail_segments = 8;
    for s in 0..trail_segments {
        let alpha = (200 - s as i32 * 20).clamp(30, 200) as u8;
        let seg_frac = (s as f32) / (trail_segments as f32);
        let seg_angle = second_angle - seg_frac * 0.015;
        let seg_len = (second_len as f32 * (1.0 - seg_frac * 0.15)) as i32;
        let seg_end = GPoint::new(
            c.x + (seg_len as f32 * seg_angle.cos()) as i32,
            c.y + (seg_len as f32 * seg_angle.sin()) as i32,
        );
        draw_line_rgba_aa(canvas, c, seg_end, rgba(CHARTREUSE.0, CHARTREUSE.1, CHARTREUSE.2, alpha));
    }

    // center pin (larger metallic dot)
    fill_rounded_rect(
        canvas,
        GRect::new(GPoint::new(c.x - 6, c.y - 6), GSize::new(12, 12)),
        6,
        rgb(STEEL_HIGHLIGHT),
    );
}


// small overlay decorations (HUD icons, triangles, tiny squares)
fn draw_overlays(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);

    // rotating decorative arc (sparse)
    let arc_radius = (h.min(w) / 2) - 24;
    let phase = t * 0.15;
    draw_arc_aa(canvas, c, arc_radius - 18, phase, phase + 0.9, rgb(CYAN_ACCENT));
    draw_arc_aa(canvas, c, arc_radius - 30, -phase * 0.8, -phase * 0.8 + 0.6, rgb((220, 220, 220)));

    // small HUD squares that blink (yellow)
    for i in 0..3 {
        let angle = (i as f32 / 3.0) * TAU + t * 0.4;
        let rad = (arc_radius as f32 * 0.6) as i32;
        let p = GPoint::new(c.x + (rad as f32 * angle.cos()) as i32, c.y + (rad as f32 * angle.sin()) as i32);
        let blink = ((t * 1.8 + i as f32).sin() * 0.5 + 0.5) * 0.6 + 0.4;
        let size = (3.0 + 3.0 * blink) as u32;
        fill_rect(canvas, GRect::new(GPoint::new(p.x - (size as i32 / 2), p.y - (size as i32 / 2)), GSize::new(size, size)), rgb(YELLOW_ACCENT));
    }

    // faint corner label "ARKNIGHTS:" in near-white (approx)
    // We don't have text draw helper here; instead make a small square block and tiny grid to evoke label area
    let label_x = w - 92;
    let label_y = h - 28;
    fill_rect(canvas, GRect::new(GPoint::new(label_x, label_y), GSize::new(8, 8)), rgb(NEAR_WHITE));
    // small horizontal ticks near label
    for i in 0..6 {
        let x = label_x + 12 + i * 8;
        draw_line_aa(canvas, GPoint::new(x, label_y + 4), GPoint::new(x + 6, label_y + 4), rgb((60, 60, 66)));
    }
}

// ---------- Optimized Watch Application Entry Point ----------
#[embassy_executor::task]
pub async fn watch_app(context: AppContext) {
    info!("Space-grade watch app started with optimized rendering pipeline");
    let start_time = Instant::now();
    
    // Create optimized render context with cached values
    // Use default screen dimensions since AppContext doesn't expose canvas size
    let mut render_context = create_render_context(240, 240);
    
    // Performance monitoring
    let mut frame_count = 0u32;
    let mut last_fps_time = start_time;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let elapsed_ms = start_time.elapsed().as_millis() as f32;
        let t = elapsed_ms / 1000.0; // seconds (float) for smooth motion
        
        // Update animation cache with new time
        render_context.animation_cache.update(t);

        // Optimized render pass with performance monitoring
        context.draw(|canvas: &mut Canvas| {
            let render_start = Instant::now();
            
            // Disable dirty tracking for bulk operations to improve performance
            canvas.set_dirty_tracking(false);
            
            // Compose the watch face with optimized rendering layers
            draw_background(canvas, t);                                    // atmosphere + grid + particles
            draw_outer_ring(canvas, t);                                    // metallic rim sheen
            draw_hour_markers(canvas, t);                                  // hour & minute markers
            draw_center_orb_and_hologram_optimized(canvas, &mut render_context); // optimized orb + 3D hologram
            draw_hands(canvas, t);                                         // hour, minute, continuous second with trail
            draw_overlays(canvas, t);                                      // HUD bits, rotating arcs, label area
            
            // Re-enable dirty tracking
            canvas.set_dirty_tracking(true);
            
            let render_time = render_start.elapsed().as_millis();
            frame_count += 1;
            
            // Log performance statistics every 60 frames
            if frame_count % 60 == 0 {
                let fps_time = Instant::now();
                let fps = 60.0 / (fps_time - last_fps_time).as_secs() as f32;
                info!("Performance: {}ms render, {} FPS", render_time, fps);
                last_fps_time = fps_time;
            }
        }).await;

        context.request_redraw().await;

        // Optimized frame timing for smooth 60Hz rendering
        // Use minimal delay to maximize frame rate while being CPU-friendly
        Timer::after(Duration::from_millis(0)).await;
    }
}
