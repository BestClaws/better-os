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
    fill_rect(canvas, GRect::new(GPoint::new(20, 20), GSize::new(100, 60)), Rgb565::from_rgb(200, 60, 60));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(18, 18), GSize::new(104, 64)), 2, Rgb565::WHITE);
    fill_rect(canvas, GRect::new(GPoint::new(150, 40), GSize::new(120, 40)), Rgb565::from_rgb(60, 150, 220));
}

fn draw_rounded_rects(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 12));
    fill_rounded_rect(canvas, GRect::new(GPoint::new(30, 40), GSize::new(120, 60)), 12, Rgb565::from_rgb(50, 180, 90));
    draw_rect_outline_aa(canvas, GRect::new(GPoint::new(28, 38), GSize::new(124, 64)), 1, Rgb565::WHITE);
}

fn draw_arcs(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let c = GPoint::new(160, 120);
    draw_arc_aa(canvas, c, 70, 0.0, core::f32::consts::PI * 1.5, Rgb565::from_rgb(255, 180, 0));
    draw_arc_aa(canvas, c, 50, -core::f32::consts::PI, core::f32::consts::PI / 2.0, Rgb565::from_rgb(0, 200, 255));
    draw_arc_aa(canvas, c, 30, core::f32::consts::PI / 3.0, core::f32::consts::PI * 1.8, Rgb565::from_rgb(120, 255, 120));
}

fn draw_lines(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(8, 8, 10));
    let c = GPoint::new(160, 120);
    for i in 0..16 {
        let ang = i as f32 / 16.0 * core::f32::consts::TAU;
        let x = c.x + (100.0 * ang.cos()) as i32;
        let y = c.y + (60.0 * ang.sin()) as i32;
        draw_line_aa(canvas, c, GPoint::new(x, y), Rgb565::from_rgb(200, 200, 200));
    }
    draw_line_rgba_aa(canvas, GPoint::new(20, 200), GPoint::new(300, 220), Rgba8888::new(255, 0, 0, 140));
}

fn draw_grad_linear(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let rect = GRect::new(GPoint::new(40, 40), GSize::new(240, 80));
    let grad = LinearGradient { start: GPoint::new(40, 40), end: GPoint::new(280, 120), start_color: Rgb565::from_rgb(255, 0, 0), end_color: Rgb565::from_rgb(0, 0, 255) };
    fill_rect_linear_gradient(canvas, rect, &grad);
}

fn draw_grad_radial(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(0, 0, 0));
    let rect = GRect::new(GPoint::new(60, 60), GSize::new(200, 120));
    let grad = RadialGradient { center: GPoint::new(160, 120), radius: 80, inner_color: Rgb565::from_rgb(255, 255, 0), outer_color: Rgb565::from_rgb(0, 0, 0) };
    fill_rect_radial_gradient(canvas, rect, &grad);
}

fn draw_alpha(canvas: &mut Canvas) {
    canvas.clear_rgb(Rgb565::from_rgb(10, 10, 10));
    fill_rect_rgba(canvas, GRect::new(GPoint::new(60, 80), GSize::new(120, 80)), Rgba8888::new(255, 0, 0, 128));
    fill_rect_rgba(canvas, GRect::new(GPoint::new(120, 120), GSize::new(120, 80)), Rgba8888::new(0, 0, 255, 128));
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