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

const TAU: f32 = 2.0 * PI;

// ---------- Palette (from your references) ----------
const BASE_DARK: (u8, u8, u8) = (12, 12, 14);
const MID_GRAY: (u8, u8, u8) = (32, 34, 38);
const STEEL: (u8, u8, u8) = (140, 145, 150);
const STEEL_HIGHLIGHT: (u8, u8, u8) = (190, 195, 200);
const YELLOW_ACCENT: (u8, u8, u8) = (245, 220, 70);
const CHARTREUSE: (u8, u8, u8) = (200, 210, 60);
const CYAN_ACCENT: (u8, u8, u8) = (120, 210, 255);
const ORANGE_ACCENT: (u8, u8, u8) = (235, 140, 60);
const NEAR_WHITE: (u8, u8, u8) = (240, 240, 240);

// Utility helpers
#[inline(always)]
fn rgb(c: (u8, u8, u8)) -> Rgb565 {
    Rgb565::from_rgb(c.0, c.1, c.2)
}
#[inline(always)]
fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8888 {
    Rgba8888::new(r, g, b, a)
}

// ---------- 3D model loader (reuse asset) ----------
fn load_demo_model() -> Model {
    let bytes = include_bytes!("../assets/geofix.stl");
    let mut model = Model::new();
    let mut vertex_map: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];
    let _ = parse_binary_stl_into(bytes, &mut model, &mut vertex_map);
    model
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

fn draw_center_orb_and_hologram(canvas: &mut Canvas, t: f32, model: &Model) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let orb_r = (w.min(h) as f32 * 0.18) as u32;

    // small radial gradient disc for orb
    let orb_grad = RadialGradient {
        center: c,
        radius: orb_r * 2,
        inner_color: rgb((68, 72, 80)), // darker metal inner
        outer_color: rgb(BASE_DARK),    // fade to background
    };
    fill_rect_radial_gradient(canvas, GRect::new(GPoint::new(c.x - orb_r as i32, c.y - orb_r as i32), GSize::new((orb_r * 2) as u32, (orb_r * 2) as u32)), &orb_grad);

    // subtle ring outline around orb
    draw_arc_aa(canvas, c, (orb_r as i32 + 8) as i32, 0.0, TAU, rgb(STEEL_HIGHLIGHT));
    draw_arc_aa(canvas, c, (orb_r as i32 + 6) as i32, 0.0, TAU, rgb(STEEL));

    // holographic 3D model inside orb (small scale)
    // keep rotations slow and subtle
    let t_mod = t * 0.6;
    let rot = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), t_mod * 0.4)
        .mul(Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), t_mod * 0.13));
    let light_dir = Quaternion::from_axis_angle(Vec3(0.6, 0.3, 0.0).normalize(), t_mod * 0.8).rotate_vector(Vec3(0.4, -0.6, -1.0));
    let origin = Vec3(0.0, 0.0, 1.2);

    let opts = RenderOptions {
        fov_deg: 35.0,
        light_dir,
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

    // draw model centered and scaled to the orb area
    // canvas coords passed to draw_model allow it to fit into the whole canvas, so we rely on opts fov and origin for scale.
    draw_model(canvas, model, origin, rot, canvas.width(), canvas.height(), &opts);

    // holographic scanlines across orb (subtle)
    for i in 0..6 {
        let offset = ((t * 0.6) + i as f32 * 0.4).sin() * (orb_r as f32 * 0.5);
        let y = c.y + offset as i32 - (orb_r as i32 / 2);
        draw_line_aa(canvas, GPoint::new(c.x - orb_r as i32, y), GPoint::new(c.x + orb_r as i32, y), rgb((28, 30, 36)));
    }
}

fn draw_hands(canvas: &mut Canvas, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let c = GPoint::new(w / 2, h / 2);
    let radius = (h.min(w) / 2) - 40;

    // compute continuous time components
    let seconds_full = t % 60.0;
    let minutes_full = (t / 60.0) % 60.0;
    let hours_full = (t / 3600.0) % 12.0;

    let hour_angle = (hours_full + minutes_full / 60.0) * (TAU / 12.0) - PI / 2.0;
    let minute_angle = (minutes_full) * (TAU / 60.0) - PI / 2.0;
    let second_angle = (seconds_full) * (TAU / 60.0) - PI / 2.0;

    // hour hand (thicker, orange)
    let hour_len = (radius as f32 * 0.48) as i32;
    let hour_end = GPoint::new(c.x + (hour_len as f32 * hour_angle.cos()) as i32, c.y + (hour_len as f32 * hour_angle.sin()) as i32);
    // draw hour base thicker by drawing two overlapping lines (steel base + orange strip)
    draw_line_aa(canvas, c, hour_end, rgb(STEEL));
    draw_line_aa(canvas, c, hour_end, rgb(ORANGE_ACCENT));

    // minute hand (sleek steel with cyan highlight)
    let minute_len = (radius as f32 * 0.72) as i32;
    let minute_end = GPoint::new(c.x + (minute_len as f32 * minute_angle.cos()) as i32, c.y + (minute_len as f32 * minute_angle.sin()) as i32);
    draw_line_aa(canvas, c, minute_end, rgb(STEEL_HIGHLIGHT));
    // cyan highlight strip (slightly offset)
    let highlight_offset = 2;
    let highlight_start = GPoint::new(c.x + highlight_offset, c.y + highlight_offset);
    let highlight_end = GPoint::new(minute_end.x + highlight_offset, minute_end.y + highlight_offset);
    draw_line_aa(canvas, highlight_start, highlight_end, rgb(CYAN_ACCENT));

    // second hand (continuous sweep) with trailing ghost using RGBA lines (fading)
    let second_len = (radius as f32 * 0.9) as i32;
    let second_tip = GPoint::new(c.x + (second_len as f32 * second_angle.cos()) as i32, c.y + (second_len as f32 * second_angle.sin()) as i32);

    // trailing ghost segments (N segments behind the tip)
    let trail_segments = 6;
    for s in 0..trail_segments {
        let alpha = ((60 - (s as i32 * 8)) as i32).clamp(12, 60) as u8; // decreasing alpha
        let seg_frac = (s as f32) / (trail_segments as f32);
        // angle slightly behind current second by small fraction (gives soft trail)
        let seg_angle = second_angle - seg_frac * 0.02;
        let seg_len = (second_len as f32 * (1.0 - seg_frac * 0.2)) as i32;
        let seg_end = GPoint::new(c.x + (seg_len as f32 * seg_angle.cos()) as i32, c.y + (seg_len as f32 * seg_angle.sin()) as i32);
        // use RGBA thin line for trail
        draw_line_rgba_aa(canvas, c, seg_end, rgba(CHARTREUSE.0 as u8, CHARTREUSE.1 as u8, CHARTREUSE.2 as u8, alpha));
    }

    // center pin (metallic)
    fill_rounded_rect(canvas, GRect::new(GPoint::new(c.x - 4, c.y - 4), GSize::new(8, 8)), 4, rgb(STEEL_HIGHLIGHT));
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

// ---------- Full watch task (entry point) ----------
#[embassy_executor::task]
pub async fn watch_app(context: AppContext) {
    info!("Watch app (aesthetic) started");
    let start_time = Instant::now();
    let model = load_demo_model();

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let elapsed_ms = start_time.elapsed().as_millis() as f32;
        let t = elapsed_ms / 1000.0; // seconds (float) for smooth motion

        // Render pass
        context.draw(|canvas: &mut Canvas| {
            // Compose the whole face from layers. We avoid changing your existing draw_* functions.
            draw_background(canvas, t);                         // atmosphere + grid + particles
            draw_outer_ring(canvas, t);                         // metallic rim sheen
            draw_hour_markers(canvas, t);                       // hour & minute markers
            draw_center_orb_and_hologram(canvas, t, &model);    // orb + 3D hologram inside
            draw_hands(canvas, t);                              // hour, minute, continuous second with trail
            draw_overlays(canvas, t);                           // HUD bits, rotating arcs, label area
        }).await;

        context.request_redraw().await;

        // Smooth continuous second hand — aim for 60Hz but keep reasonable on embedded; 16ms ~ 62.5Hz
        Timer::after(Duration::from_millis(0)).await;
    }
}
