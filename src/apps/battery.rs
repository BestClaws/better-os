#![allow(unused)]
use defmt::info;
use embassy_time::{Duration, Timer, Instant};
use micromath::F32Ext;
use crate::system::ui::gfx::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, Rgba8888, LinearGradient, RadialGradient,
    draw_line_aa, draw_line_rgba_aa, draw_arc_aa, fill_rect, draw_rect_outline_aa, fill_rounded_rect,
    fill_rect_linear_gradient, fill_rect_radial_gradient, fill_rect_rgba};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;

const CANVAS_WIDTH: i32 = 320;
const CANVAS_HEIGHT: i32 = 240;

#[derive(Clone, Copy)]
enum DemoScene { Rects, RoundedRects, Arcs, Lines, GradLinear, GradRadial, Alpha }

impl DemoScene {
    fn next(self) -> Self { match self { DemoScene::Rects => DemoScene::RoundedRects, DemoScene::RoundedRects => DemoScene::Arcs, DemoScene::Arcs => DemoScene::Lines, DemoScene::Lines => DemoScene::GradLinear, DemoScene::GradLinear => DemoScene::GradRadial, DemoScene::GradRadial => DemoScene::Alpha, DemoScene::Alpha => DemoScene::Rects } }
}

fn draw_rects(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(15, 15, 20));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 8;
    let w1 = (cw / 3).max(20);
    let h1 = (ch / 4).max(20);
    fill_rect(canvas, GRect::new(GPoint::new(m, m), GSize::new(w1 as u32, h1 as u32)), Rgb565::from_rgb(200, 60, 60));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(m - 2, m - 2), GSize::new((w1 + 4) as u32, (h1 + 4) as u32)), 2, Rgb565::WHITE);
    let w2 = (cw / 2 - 2 * m).max(20);
    let h2 = (ch / 6).max(16);
    let x2 = (cw - w2 - m).max(m);
    let y2 = (ch / 3).clamp(m, ch - h2 - m);
    fill_rect(canvas, GRect::new(GPoint::new(x2, y2), GSize::new(w2 as u32, h2 as u32)), Rgb565::from_rgb(60, 150, 220));
}

fn draw_rounded_rects(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 12));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 10;
    let w = (cw - 2 * m).max(20);
    let h = (ch / 3).max(20);
    let y = (ch / 2 - h / 2).max(m);
    let r = (h / 4).max(4);
    fill_rounded_rect(canvas, GRect::new(GPoint::new(m, y), GSize::new(w as u32, h as u32)), r, Rgb565::from_rgb(50, 180, 90));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(m - 2, y - 2), GSize::new((w + 4) as u32, (h + 4) as u32)), 1, Rgb565::WHITE);
}

fn draw_arcs(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let c = GPoint::new(cw / 2, ch / 2);
    let max_r = (cw.min(ch) / 2) - 12;
    let r1 = (max_r).max(12);
    let r2 = (max_r * 2 / 3).max(8);
    let r3 = (max_r / 2).max(6);
    draw_arc_aa(canvas, c, r1, 0.0, core::f32::consts::PI * 1.5, Rgb565::from_rgb(255, 180, 0));
    draw_arc_aa(canvas, c, r2, -core::f32::consts::PI, core::f32::consts::PI / 2.0, Rgb565::from_rgb(0, 200, 255));
    draw_arc_aa(canvas, c, r3, core::f32::consts::PI / 3.0, core::f32::consts::PI * 1.8, Rgb565::from_rgb(120, 255, 120));
}

fn draw_lines(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(8, 8, 10));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let c = GPoint::new(cw / 2, ch / 2);
    let rx = (cw as f32 * 0.35).max(20.0);
    let ry = (ch as f32 * 0.25).max(16.0);
    for i in 0..16 {
        let ang = i as f32 / 16.0 * core::f32::consts::TAU;
        let x = c.x + (rx * ang.cos()) as i32;
        let y = c.y + (ry * ang.sin()) as i32;
        draw_line_aa(canvas, c, GPoint::new(x, y), Rgb565::from_rgb(200, 200, 200));
    }
    let y1 = (ch as f32 * 0.85) as i32;
    let y2 = (ch as f32 * 0.92) as i32;
    draw_line_rgba_aa(canvas, GPoint::new(10, y1), GPoint::new(cw - 10, y2), Rgba8888::new(255, 0, 0, 140));
}

fn draw_grad_linear(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let rect = GRect::new(GPoint::new(m, ch / 2 - ch / 6), GSize::new((cw - 2 * m) as u32, (ch / 3) as u32));
    let grad = LinearGradient { start: GPoint::new(m, ch / 2 - ch / 6), end: GPoint::new(cw - m, ch / 2 + ch / 6), start_color: Rgb565::from_rgb(255, 0, 0), end_color: Rgb565::from_rgb(0, 0, 255) };
    fill_rect_linear_gradient(canvas, rect, &grad);
}

fn draw_grad_radial(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let rect = GRect::new(GPoint::new(m, m), GSize::new((cw - 2 * m) as u32, (ch - 2 * m) as u32));
    let r = (cw.min(ch) / 3).max(16) as u32;
    let grad = RadialGradient { center: GPoint::new(cw / 2, ch / 2), radius: r, inner_color: Rgb565::from_rgb(255, 255, 0), outer_color: Rgb565::from_rgb(0, 0, 0) };
    fill_rect_radial_gradient(canvas, rect, &grad);
}

fn draw_alpha(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 10));
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let m = 12;
    let w = (cw / 3).max(40);
    let h = (ch / 3).max(40);
    let x1 = m;
    let y1 = (ch / 2 - h / 2).max(m);
    let x2 = (x1 + w / 2).min(cw - w - m);
    let y2 = (y1 + h / 2).min(ch - h - m);
    fill_rect_rgba(canvas, GRect::new(GPoint::new(x1, y1), GSize::new(w as u32, h as u32)), Rgba8888::new(255, 0, 0, 128));
    fill_rect_rgba(canvas, GRect::new(GPoint::new(x2, y2), GSize::new(w as u32, h as u32)), Rgba8888::new(0, 0, 255, 128));
}

#[embassy_executor::task]
pub async fn battery_app(context: AppContext) {
    info!("Graphics demo started");
    let mut scene = DemoScene::Rects;
    let mut last_switch = Instant::now();

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        if last_switch.elapsed() >= Duration::from_secs(2) {
            scene = scene.next();
            last_switch = Instant::now();
        }

        context.draw(|canvas: &mut Canvas| {
            match scene {
                DemoScene::Rects => draw_rects(canvas),
                DemoScene::RoundedRects => draw_rounded_rects(canvas),
                DemoScene::Arcs => draw_arcs(canvas),
                DemoScene::Lines => draw_lines(canvas),
                DemoScene::GradLinear => draw_grad_linear(canvas),
                DemoScene::GradRadial => draw_grad_radial(canvas),
                DemoScene::Alpha => draw_alpha(canvas),
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await;
    }
}