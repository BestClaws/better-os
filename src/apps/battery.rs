#![allow(unused)]
use defmt::{debug, info};
use embassy_time::{Duration, Timer, Instant};
use micromath::F32Ext;
use crate::libs::gfx::math::Quaternion;
use crate::libs::gfx::{Model, Vec3};
use crate::libs::gfx::three_d::model::{parse_binary_stl_into, MAX_VERTICES};
use crate::libs::gfx::three_d::render::{draw_model, RenderOptions, ShadingMode, AntiAliasing, ViewMode, LightingMode};
use crate::libs::gfx::two_d::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, Rgba8888,
                              Draw, FillStyle, CornerRadiiPx, gradient, gradient_vertical, gradient_angle, radial,
                              TextRenderer, FONT_8X8};
fn draw_3d_demo(canvas: &mut Canvas, t: f32, model: &Model) {
    let now = Instant::now();
    let (rot, light_dir) =  {
        let dt = t;
        let rot_y = Quaternion::from_axis_angle(Vec3(0.0, 1.0, 0.0), dt * 0.7);
        let rot_x = Quaternion::from_axis_angle(Vec3(1.0, 0.0, 0.0), dt * 0.3);
        let rot = rot_y.mul(rot_x);
        let light_dir = Vec3(0.0, 0.0, -1.0);
        (rot, light_dir)
    };
    // Position the model slightly in front of camera (camera looks +Z)
    let origin = Vec3(0.0, 0.0, 1.0);
    // Restore original two-phase demo (showcase behavior controlled elsewhere)
    let opts = RenderOptions {
        // Vertical field-of-view in degrees (smaller narrows perspective, larger widens it)
        fov_deg: 40.0,
        // Direction the directional light comes FROM in camera space (unit length recommended)
        // Camera looks along +Z, so a vector with negative Z lights faces pointing to camera.
        light_dir,
        // Grayscale-only intensity clamp for computed lighting in [min, max].
        // This does not affect colored lighting; it is used by grayscale helpers.
        intensity_range: (0.2, 1.0),
        // If true, triangles whose face normal points away from camera are skipped.
        enable_backface_culling: true,
        // If true, triangles are painter-sorted by average camera-space Z before drawing.
        enable_depth_sorting: true,
        // Lighting mode selection
        lighting_mode: LightingMode::AmbientAndDirectional,
        // If true, clip geometry against a near plane at `near_z` in camera space.
        enable_near_clipping: true,
        // If true, drop projected points outside the viewport (cheap bounds cull).
        enable_frustum_clipping: false,
        // View mode
        view_mode: ViewMode::Fill,
        // Near plane distance for clipping; only used when `enable_near_clipping` is true.
        near_z: 0.1,
        // Shading mode
        shading_mode: ShadingMode::Flat,
        // Anti-aliasing mode
        aa_mode: AntiAliasing::None,
        // Ambient light color (blue-tinted to distinguish from model)
        ambient_color: Rgb565::from_rgb(80, 120, 255),
        // Directional light color (red to be obvious)
        directional_color: Rgb565::from_rgb(255, 80, 80),
        // Base model surface color (white so lighting colors are visible)
        model_color: Rgb565::from_rgb(255, 255, 255),
    };
    draw_model(canvas, model, origin, rot, canvas.width(), canvas.height(), &opts);



    debug!("3d demo frame time: {}", now.elapsed().as_millis());
}
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use alloc::format;

const CANVAS_WIDTH: i32 = 320;
const CANVAS_HEIGHT: i32 = 240;

#[derive(Clone, Copy)]
enum DemoScene { Rects, RoundedRects, NonUniformCorners, Arcs, Lines, PolyLine, Beziers, GradLinear, GradRadial, Alpha, ThreeD }

impl DemoScene {
    fn next(self) -> Self {
        match self {
            DemoScene::Rects => DemoScene::RoundedRects,
            DemoScene::RoundedRects => DemoScene::NonUniformCorners,
            DemoScene::NonUniformCorners => DemoScene::Arcs,
            DemoScene::Arcs => DemoScene::Lines,
            DemoScene::Lines => DemoScene::PolyLine,
            DemoScene::PolyLine => DemoScene::Beziers,
            DemoScene::Beziers => DemoScene::GradLinear,
            DemoScene::GradLinear => DemoScene::GradRadial,
            DemoScene::GradRadial => DemoScene::Alpha,
            DemoScene::Alpha => DemoScene::ThreeD,
            DemoScene::ThreeD => DemoScene::Rects,
        }
    }
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
    Draw::new(canvas)
        .rect(GRect::new(GPoint::new(m, m), GSize::new(w1 as u32, h1 as u32)))
        .fill_color(Rgb565::from_rgb(200, 60, 60))
        .draw();
    Draw::new(canvas)
        .rect(GRect::new(GPoint::new(m - 2, m - 2), GSize::new((w1 + 4) as u32, (h1 + 4) as u32)))
        .stroke(crate::libs::gfx::two_d::stroke(2, Rgb565::WHITE))
        .draw();
    let w2 = (cw / 2 - 2 * m).max(20);
    let h2 = (ch / 6).max(16);
    let x2_center = (cw - w2 - m).max(m) + w2 / 2;
    let x2 = (x2_center as f32 + (cw as f32 * 0.08) * (t * 1.3).sin()) as i32 - w2 / 2;
    let y2 = (ch / 3).clamp(m, ch - h2 - m);
    Draw::new(canvas)
        .rect(GRect::new(GPoint::new(x2, y2), GSize::new(w2 as u32, h2 as u32)))
        .fill_color(Rgb565::from_rgb(60, 150, 220))
        .draw();
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
    Draw::new(canvas)
        .rect(GRect::new(GPoint::new(m, y), GSize::new(w as u32, h as u32)))
        .corner_radius(r)
        .fill_color(Rgb565::from_rgb(50, 180, 90))
        .draw();
    Draw::new(canvas)
        .rect(GRect::new(GPoint::new(m - 2, y - 2), GSize::new((w + 4) as u32, (h + 4) as u32)))
        .stroke(crate::libs::gfx::two_d::stroke(1, Rgb565::WHITE))
        .draw();
}

fn draw_non_uniform_corners(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(8, 10, 12));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let w = (cw - 2 * m).max(40);
    let h = (ch - 2 * m).max(40);
    let rect = GRect::new(GPoint::new(m, m), GSize::new(w as u32, h as u32));
    let r_t = (12.0 + 8.0 * (t * 1.3).sin()).abs() as i32;
    let r_r = (18.0 + 10.0 * (t * 0.9).cos()).abs() as i32;
    let r_b = (8.0 + 6.0 * (t * 1.7).sin()).abs() as i32;
    let r_l = (20.0 + 12.0 * (t * 1.1).cos()).abs() as i32;
    let radii = CornerRadiiPx { tl: r_t, tr: r_r, br: r_b, bl: r_l };
    Draw::new(canvas)
        .rect(rect)
        .corner_radii(radii)
        .fill(FillStyle::Linear(gradient_vertical(Rgb565::from_rgb(30, 60, 180), Rgb565::from_rgb(10, 20, 80))))
        .draw();
    Draw::new(canvas)
        .rect(rect)
        .corner_radii(radii)
        .stroke(crate::libs::gfx::two_d::stroke(2, Rgb565::WHITE))
        .draw();
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
    Draw::new(canvas).arc(c, r1, 0.0, sweep1).color(Rgb565::from_rgb(255, 180, 0)).draw();
    Draw::new(canvas).arc(c, r2, start2, end2).color(Rgb565::from_rgb(0, 200, 255)).draw();
    Draw::new(canvas).arc(c, r3, core::f32::consts::PI / 3.0 + off3, core::f32::consts::PI * 1.8 + off3).color(Rgb565::from_rgb(120, 255, 120)).draw();
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
        Draw::new(canvas).line(c, GPoint::new(x, y)).color(Rgb565::from_rgb(200, 200, 200)).draw();
    }
    let y1 = (ch as f32 * (0.8 + 0.05 * (t * 1.1).sin())) as i32;
    let y2 = (ch as f32 * (0.9 + 0.05 * (t * 1.1).cos())) as i32;
    Draw::new(canvas).line(GPoint::new(10, y1), GPoint::new(cw - 10, y2)).color_rgba(Rgba8888::new(255, 0, 0, 140)).draw();
}

fn draw_polyline(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(6, 6, 8));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let cx = cw / 2;
    let cy = ch / 2;
    let r = (ch.min(cw) as f32 * (0.3 + 0.05 * (t * 0.7).sin())) as i32;
    let n = 8;
    let mut d = Draw::new(canvas);
    let mut path = d.path().stroke(crate::libs::gfx::two_d::stroke(2, Rgb565::from_rgb(220, 220, 240)));
    for i in 0..n {
        let ang = (i as f32 / n as f32) * core::f32::consts::TAU + t * 0.4;
        let px = cx + (r as f32 * ang.cos()) as i32;
        let py = cy + (r as f32 * ang.sin()) as i32;
        if i == 0 { path = path.move_to(GPoint::new(px, py)); }
        else { path = path.line_to(GPoint::new(px, py)); }
    }
    path.close().finish();
}

fn draw_beziers(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(4, 6, 10));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let p0 = GPoint::new(cw / 6, ch / 2);
    let p1 = GPoint::new(cw * 5 / 6, ch / 2);
    let c = GPoint::new(cw / 2, (ch as f32 * (0.3 + 0.15 * (t * 1.1).sin())) as i32);
    let c1 = GPoint::new((cw as f32 * (0.3 + 0.1 * (t * 0.8).cos())) as i32, ch / 3);
    let c2 = GPoint::new((cw as f32 * (0.7 + 0.1 * (t * 0.8).sin())) as i32, ch * 2 / 3);

    // Quadratic Bezier
    Draw::new(canvas)
        .path()
        .stroke(crate::libs::gfx::two_d::stroke(2, Rgb565::from_rgb(255, 120, 80)))
        .move_to(p0)
        .quadratic_to(c, p1)
        .finish();

    // Cubic Bezier
    Draw::new(canvas)
        .path()
        .stroke(crate::libs::gfx::two_d::stroke(2, Rgb565::from_rgb(80, 220, 255)))
        .move_to(p0)
        .cubic_to(c1, c2, p1)
        .finish();
}

fn draw_grad_linear(canvas: &mut Canvas, t: f32) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let rect = GRect::new(GPoint::new(m, ch / 2 - ch / 6), GSize::new((cw - 2 * m) as u32, (ch / 3) as u32));
    let end_x = cw - m - ((cw as f32 * 0.1) * (t * 0.7).sin()) as i32;
    let end_y = ch / 2 + ch / 6 + ((ch as f32 * 0.05) * (t * 0.9).cos()) as i32;
    Draw::new(canvas)
        .rect(rect)
        .fill(FillStyle::Linear(gradient_angle(30.0, Rgb565::from_rgb(255, 0, 0), Rgb565::from_rgb(0, 0, 255))))
        .draw();
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
    Draw::new(canvas)
        .rect(rect)
        .fill(FillStyle::Radial(radial(GPoint::new(cx, cy), r, Rgb565::from_rgb(255, 255, 0), Rgb565::from_rgb(0, 0, 0))))
        .draw();
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
    Draw::new(canvas).rect(GRect::new(GPoint::new(x1, y1), GSize::new(w as u32, h as u32))).fill(FillStyle::Rgba(Rgba8888::new(255, 0, 0, 128))).draw();
    Draw::new(canvas).rect(GRect::new(GPoint::new(x2, y2), GSize::new(w as u32, h as u32))).fill(FillStyle::Rgba(Rgba8888::new(0, 0, 255, 128))).draw();
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    debug!("Graphics demo started");
    let mut scene = DemoScene::Rects;
    let mut last_switch = Instant::now();
    let start_time = Instant::now();
    // Load model once safely; reuse across frames.
    let bytes = include_bytes!("../assets/geofix.stl");
    let mut model = Model::new();
    let mut vertex_map: [Option<usize>; MAX_VERTICES] = [None; MAX_VERTICES];
    let _ = parse_binary_stl_into(bytes, &mut model, &mut vertex_map);

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
                DemoScene::NonUniformCorners => draw_non_uniform_corners(canvas, t),
                DemoScene::Arcs => draw_arcs(canvas, t),
                DemoScene::Lines => draw_lines(canvas, t),
                DemoScene::PolyLine => draw_polyline(canvas, t),
                DemoScene::Beziers => draw_beziers(canvas, t),
                DemoScene::GradLinear => draw_grad_linear(canvas, t),
                DemoScene::GradRadial => draw_grad_radial(canvas, t),
                DemoScene::Alpha => draw_alpha(canvas, t),
                _=> draw_3d_demo(canvas, t, &model),
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(1)).await;
    }
}