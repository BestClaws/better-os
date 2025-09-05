#![allow(unused)]
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use micromath::F32Ext;
use crate::libs::gfx::two_d::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, Rgba8888, LinearGradient, RadialGradient,
    draw_line_aa, draw_line_rgba_aa, draw_arc_aa, fill_rect, draw_rect_outline_aa, fill_rounded_rect,
    fill_rect_linear_gradient, fill_rect_radial_gradient, fill_rect_rgba};
use crate::libs::gfx::three_d::{Model, Quaternion, Vec3, RenderOptions, draw_model, parse_binary_stl_into, MAX_VERTICES};
fn draw_3d_demo(canvas: &mut Canvas, t: f32) {
    // Load and cache the STL model statically
    static mut MODEL: Option<Model> = None;
    static mut MAP: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];
    unsafe {
        if MODEL.is_none() {
            // Embedded asset path; parse_binary_stl expects &[u8]
            let bytes = include_bytes!("../assets/geofix.stl");
            let mut model = Model::new();
            if parse_binary_stl_into(bytes, &mut model, &mut MAP).is_ok() {
                MODEL = Some(model);
            }
        }
        if let Some(model) = &MODEL {
            // Clear background
            canvas.clear_rgb(Rgb565::from_rgb(4, 4, 8));
            // Compute rotation
            // Compose rotations around Y and X axes
            let rot_y = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), t * 0.7);
            let rot_x = Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), t * 0.3);
            let rot = rot_y.mul(rot_x);
            // Position the model slightly in front of camera
            let origin = Vec3(0.0, 0.0, 3.0);
            let opts = RenderOptions {
                fov_deg: 60.0,
                light_dir: Vec3(0.3, 0.6, 1.0),
                intensity_range: (0.2, 1.0),
                enable_backface_culling: true,
                enable_zbuffer: false,
                enable_lighting: true,
                enable_depth_sorting: true,
                enable_near_clipping: true,
                enable_frustum_clipping: false,
                enable_wireframe: false,
                enable_shading: true,
                enable_antialiasing: true,
                antialiasing_factor: 1,
                edge_only_antialiasing: true,
            };
            draw_model(canvas, model, origin, rot, canvas.width(), canvas.height(), &opts);
        }
    }
}
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;

const CANVAS_WIDTH: i32 = 320;
const CANVAS_HEIGHT: i32 = 240;

#[derive(Clone, Copy)]
enum DemoScene { Rects, RoundedRects, Arcs, Lines, GradLinear, GradRadial, Alpha, ThreeD }

impl DemoScene {
    fn next(self) -> Self { match self { DemoScene::Rects => DemoScene::RoundedRects, DemoScene::RoundedRects => DemoScene::Arcs, DemoScene::Arcs => DemoScene::Lines, DemoScene::Lines => DemoScene::GradLinear, DemoScene::GradLinear => DemoScene::GradRadial, DemoScene::GradRadial => DemoScene::Alpha, DemoScene::Alpha => DemoScene::ThreeD, DemoScene::ThreeD => DemoScene::Rects } }
}

fn draw_rects(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(15, 15, 20));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 8;
    let base_w1 = (cw / 3).max(20);
    let base_h1 = (ch / 4).max(20);
    let scale = 1.0 + 0.2 * (t * 2.0).sin();
    let w1 = ((base_w1 as f32) * scale) as i32;
    let h1 = ((base_h1 as f32) * scale) as i32;
    fill_rect(canvas, GRect::new(GPoint::new(m, m), GSize::new(w1 as u32, h1 as u32)), Rgb565::from_rgb(200, 60, 60));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(m - 2, m - 2), GSize::new((w1 + 4) as u32, (h1 + 4) as u32)), 2, Rgb565::WHITE);
    let w2 = (cw / 2 - 2 * m).max(20);
    let h2 = (ch / 6).max(16);
    let x2_center = (cw - w2 - m).max(m) + w2 / 2;
    let x2 = (x2_center as f32 + (cw as f32 * 0.08) * (t * 1.3).sin()) as i32 - w2 / 2;
    let y2 = (ch / 3).clamp(m, ch - h2 - m);
    fill_rect(canvas, GRect::new(GPoint::new(x2, y2), GSize::new(w2 as u32, h2 as u32)), Rgb565::from_rgb(60, 150, 220));
}

fn draw_rounded_rects(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 12));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 10;
    let w = (cw - 2 * m).max(20);
    let h = (ch / 3).max(20);
    let y = (ch / 2 - h / 2).max(m);
    let r_base = (h / 4).max(4);
    let r = (r_base as f32 * (0.6 + 0.4 * (t * 1.7).sin().abs())) as i32;
    fill_rounded_rect(canvas, GRect::new(GPoint::new(m, y), GSize::new(w as u32, h as u32)), r, Rgb565::from_rgb(50, 180, 90));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(m - 2, y - 2), GSize::new((w + 4) as u32, (h + 4) as u32)), 1, Rgb565::WHITE);
}

fn draw_arcs(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let c = GPoint::new(cw / 2, ch / 2);
    let max_r = (cw.min(ch) / 2) - 12;
    let r1 = (max_r).max(12);
    let r2 = (max_r * 2 / 3).max(8);
    let r3 = (max_r / 2).max(6);
    let sweep1 = core::f32::consts::PI * (1.0 + 0.5 * (t * 0.8).sin());
    let start2 = -core::f32::consts::PI + 0.5 * (t * 0.9).cos();
    let end2 = start2 + core::f32::consts::PI / 2.0 + 0.5 * (t * 0.9).sin();
    let off3 = (t * 1.2).sin() * 0.5;
    draw_arc_aa(canvas, c, r1, 0.0, sweep1, Rgb565::from_rgb(255, 180, 0));
    draw_arc_aa(canvas, c, r2, start2, end2, Rgb565::from_rgb(0, 200, 255));
    draw_arc_aa(canvas, c, r3, core::f32::consts::PI / 3.0 + off3, core::f32::consts::PI * 1.8 + off3, Rgb565::from_rgb(120, 255, 120));
}

fn draw_lines(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(8, 8, 10));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let c = GPoint::new(cw / 2, ch / 2);
    let rx = (cw as f32 * 0.35).max(20.0);
    let ry = (ch as f32 * 0.25).max(16.0);
    for i in 0..16 {
        let ang = i as f32 / 16.0 * core::f32::consts::TAU + t * 0.6;
        let x = c.x + (rx * ang.cos()) as i32;
        let y = c.y + (ry * ang.sin()) as i32;
        draw_line_aa(canvas, c, GPoint::new(x, y), Rgb565::from_rgb(200, 200, 200));
    }
    let y1 = (ch as f32 * (0.8 + 0.05 * (t * 1.1).sin())) as i32;
    let y2 = (ch as f32 * (0.9 + 0.05 * (t * 1.1).cos())) as i32;
    draw_line_rgba_aa(canvas, GPoint::new(10, y1), GPoint::new(cw - 10, y2), Rgba8888::new(255, 0, 0, 140));
}

fn draw_grad_linear(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let rect = GRect::new(GPoint::new(m, ch / 2 - ch / 6), GSize::new((cw - 2 * m) as u32, (ch / 3) as u32));
    let end_x = cw - m - ((cw as f32 * 0.1) * (t * 0.7).sin()) as i32;
    let end_y = ch / 2 + ch / 6 + ((ch as f32 * 0.05) * (t * 0.9).cos()) as i32;
    let grad = LinearGradient { start: GPoint::new(m, ch / 2 - ch / 6), end: GPoint::new(end_x, end_y), start_color: Rgb565::from_rgb(255, 0, 0), end_color: Rgb565::from_rgb(0, 0, 255) };
    fill_rect_linear_gradient(canvas, rect, &grad);
}

fn draw_grad_radial(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let rect = GRect::new(GPoint::new(m, m), GSize::new((cw - 2 * m) as u32, (ch - 2 * m) as u32));
    let r = (cw.min(ch) / 3).max(16) as u32;
    let cx = cw / 2 + ((cw as f32 * 0.1) * (t * 0.6).sin()) as i32;
    let cy = ch / 2 + ((ch as f32 * 0.1) * (t * 0.6).cos()) as i32;
    let grad = RadialGradient { center: GPoint::new(cx, cy), radius: r, inner_color: Rgb565::from_rgb(255, 255, 0), outer_color: Rgb565::from_rgb(0, 0, 0) };
    fill_rect_radial_gradient(canvas, rect, &grad);
}

fn draw_alpha(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 10));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let w = (cw / 3).max(40);
    let h = (ch / 3).max(40);
    let x1 = m + ((cw as f32 * 0.05) * (t * 1.1).sin()) as i32;
    let y1 = ((ch / 2 - h / 2).max(m) as f32 + (ch as f32 * 0.05) * (t * 1.3).cos()) as i32;
    let x2 = (x1 + w / 2).clamp(m, cw - w - m);
    let y2 = (y1 + h / 2).clamp(m, ch - h - m);
    fill_rect_rgba(canvas, GRect::new(GPoint::new(x1, y1), GSize::new(w as u32, h as u32)), Rgba8888::new(255, 0, 0, 128));
    fill_rect_rgba(canvas, GRect::new(GPoint::new(x2, y2), GSize::new(w as u32, h as u32)), Rgba8888::new(0, 0, 255, 128));
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    info!("Graphics demo started");
    let mut scene = DemoScene::Rects;
    let mut last_switch = Instant::now();
    let start_time = Instant::now();

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        if last_switch.elapsed() >= Duration::from_secs(2) {
            scene = scene.next();
            last_switch = Instant::now();
        }

        let t = start_time.elapsed().as_millis() as f32 / 1000.0;
        context.draw(|canvas: &mut Canvas| {
            match scene {
                DemoScene::Rects => draw_rects(canvas, t),
                DemoScene::RoundedRects => draw_rounded_rects(canvas, t),
                DemoScene::Arcs => draw_arcs(canvas, t),
                DemoScene::Lines => draw_lines(canvas, t),
                DemoScene::GradLinear => draw_grad_linear(canvas, t),
                DemoScene::GradRadial => draw_grad_radial(canvas, t),
                DemoScene::Alpha => draw_alpha(canvas, t),
                DemoScene::ThreeD => draw_3d_demo(canvas, t),
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}