#![no_std]

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use rust_gfx::color::Rgba8888;
use rust_gfx::primitives::line::{draw_line, LineDsc};
use rust_gfx::primitives::rectangle::{draw_rect, RectDsc};
use rust_gfx::{
    Area, BorderSide, GradDir, Point, OPA_30, OPA_50, OPA_70, OPA_COVER, RADIUS_CIRCLE,
};

const SPRITE_X: i32 = 20;
const SPRITE_Y: i32 = 30;
const SPRITE_W: i32 = 63;
const SPRITE_H: i32 = 66;

#[embassy_executor::task]
pub async fn gfx_bench_app(context: AppContext) {
    info!("GFX Bench: Showcasing 48 passing LVGL-compatible sprites");

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        // 0-19: Solid rectangles with various radius and colors
        let radii = [0, 5, 10, 20, RADIUS_CIRCLE];
        let radius_names = ["r0", "r5", "r10", "r20", "rcircle"];
        let colors = [
            Rgba8888::rgb(255, 100, 100),
            Rgba8888::rgb(100, 255, 100),
            Rgba8888::rgb(100, 100, 255),
            Rgba8888::rgb(255, 255, 100),
        ];
        let color_names = ["red", "green", "blue", "yellow"];

        for r in 0..5 {
            for c in 0..4 {
                let start = Instant::now();
                context
                    .draw(|surface| {
                        surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                        let mut dsc = RectDsc::new();
                        dsc.radius = radii[r];
                        dsc.bg_opa = OPA_COVER;
                        dsc.bg_color = colors[c];
                        draw_rect(
                            surface,
                            &dsc,
                            &Area::new(
                                SPRITE_X,
                                SPRITE_Y,
                                SPRITE_X + SPRITE_W - 1,
                                SPRITE_Y + SPRITE_H - 1,
                            ),
                        );
                    })
                    .await;
                info!(
                    "rect_solid_{}_{}: {} us",
                    radius_names[r],
                    color_names[c],
                    start.elapsed().as_micros()
                );
                Timer::after(Duration::from_millis(100)).await;
            }
        }

        // 20-25: Gradients (horizontal and vertical)
        let grad_radii = [0, 5, 10];
        for &radius in grad_radii.iter() {
            // Horizontal gradient
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = RectDsc::new();
                    dsc.radius = radius;
                    dsc.bg_color = Rgba8888::rgb(255, 0, 0);
                    dsc.bg_opa = OPA_COVER;
                    dsc.bg_grad.dir = GradDir::Hor;
                    dsc.bg_grad.stops[1].color = Rgba8888::rgb(0, 0, 255);
                    draw_rect(
                        surface,
                        &dsc,
                        &Area::new(
                            SPRITE_X,
                            SPRITE_Y,
                            SPRITE_X + SPRITE_W - 1,
                            SPRITE_Y + SPRITE_H - 1,
                        ),
                        None,
                    );
                })
                .await;
            info!(
                "rect_grad_hor_r{}: {} us",
                radius,
                start.elapsed().as_micros()
            );
            Timer::after(Duration::from_millis(100)).await;
        }

        for &radius in grad_radii.iter() {
            // Vertical gradient
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = RectDsc::new();
                    dsc.radius = radius;
                    dsc.bg_color = Rgba8888::rgb(255, 0, 0);
                    dsc.bg_opa = OPA_COVER;
                    dsc.bg_grad.dir = GradDir::Ver;
                    dsc.bg_grad.stops[1].color = Rgba8888::rgb(0, 0, 255);
                    draw_rect(
                        surface,
                        &dsc,
                        &Area::new(
                            SPRITE_X,
                            SPRITE_Y,
                            SPRITE_X + SPRITE_W - 1,
                            SPRITE_Y + SPRITE_H - 1,
                        ),
                        None,
                    );
                })
                .await;
            info!(
                "rect_grad_ver_r{}: {} us",
                radius,
                start.elapsed().as_micros()
            );
            Timer::after(Duration::from_millis(100)).await;
        }

        for &radius in grad_radii.iter() {
            // Radial gradient (fallback to horizontal in simple mode)
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = RectDsc::new();
                    dsc.radius = radius;
                    dsc.bg_color = Rgba8888::rgb(255, 0, 0);
                    dsc.bg_opa = OPA_COVER;
                    dsc.bg_grad.dir = GradDir::Radial;
                    dsc.bg_grad.stops[1].color = Rgba8888::rgb(0, 0, 255);
                    draw_rect(
                        surface,
                        &dsc,
                        &Area::new(
                            SPRITE_X,
                            SPRITE_Y,
                            SPRITE_X + SPRITE_W - 1,
                            SPRITE_Y + SPRITE_H - 1,
                        ),
                        None,
                    );
                })
                .await;
            info!(
                "rect_grad_radial_r{}: {} us",
                radius,
                start.elapsed().as_micros()
            );
            Timer::after(Duration::from_millis(100)).await;
        }

        for &radius in grad_radii.iter() {
            // Conical gradient (fallback to horizontal in simple mode)
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = RectDsc::new();
                    dsc.radius = radius;
                    dsc.bg_color = Rgba8888::rgb(255, 0, 0);
                    dsc.bg_opa = OPA_COVER;
                    dsc.bg_grad.dir = GradDir::Conical;
                    dsc.bg_grad.stops[1].color = Rgba8888::rgb(0, 0, 255);
                    draw_rect(
                        surface,
                        &dsc,
                        &Area::new(
                            SPRITE_X,
                            SPRITE_Y,
                            SPRITE_X + SPRITE_W - 1,
                            SPRITE_Y + SPRITE_H - 1,
                        ),
                        None,
                    );
                })
                .await;
            info!(
                "rect_grad_conical_r{}: {} us",
                radius,
                start.elapsed().as_micros()
            );
            Timer::after(Duration::from_millis(100)).await;
        }

        // 44: Border w10 full
        let start = Instant::now();
        context
            .draw(|surface| {
                surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                let mut dsc = RectDsc::new();
                dsc.radius = 10;
                dsc.bg_color = Rgba8888::rgb(50, 50, 50);
                dsc.bg_opa = OPA_COVER;
                dsc.border_width = 10;
                dsc.border_color = Rgba8888::rgb(255, 255, 0);
                dsc.border_opa = OPA_COVER;
                dsc.border_side = BorderSide::FULL;
                draw_rect(
                    surface,
                    &dsc,
                    &Area::new(
                        SPRITE_X,
                        SPRITE_Y,
                        SPRITE_X + SPRITE_W - 1,
                        SPRITE_Y + SPRITE_H - 1,
                    ),
                    None,
                );
            })
            .await;
        info!("rect_border_w10_full: {} us", start.elapsed().as_micros());
        Timer::after(Duration::from_millis(100)).await;

        // 60-63: Opacity tests
        let opacities = [
            (OPA_COVER, "100"),
            (OPA_70, "70"),
            (OPA_50, "50"),
            (OPA_30, "30"),
        ];
        for &(opa, name) in opacities.iter() {
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = RectDsc::new();
                    dsc.radius = 10;
                    dsc.bg_color = Rgba8888::rgb(255, 0, 0);
                    dsc.bg_opa = opa;
                    draw_rect(
                        surface,
                        &dsc,
                        &Area::new(
                            SPRITE_X,
                            SPRITE_Y,
                            SPRITE_X + SPRITE_W - 1,
                            SPRITE_Y + SPRITE_H - 1,
                        ),
                        None,
                    );
                })
                .await;
            info!("rect_opa{}: {} us", name, start.elapsed().as_micros());
            Timer::after(Duration::from_millis(100)).await;
        }

        // 97-104: Horizontal and vertical lines
        let line_widths = [1, 3, 6, 10];
        for &width in line_widths.iter() {
            // Horizontal
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = LineDsc::new(Point::new(15, 62), Point::new(87, 62));
                    dsc.width = width;
                    dsc.color = Rgba8888::WHITE;
                    draw_line(surface, &dsc);
                })
                .await;
            info!("line_hor_w{}: {} us", width, start.elapsed().as_micros());
            Timer::after(Duration::from_millis(100)).await;
        }

        for &width in line_widths.iter() {
            // Vertical
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = LineDsc::new(Point::new(50, 30), Point::new(50, 95));
                    dsc.width = width;
                    dsc.color = Rgba8888::WHITE;
                    draw_line(surface, &dsc);
                })
                .await;
            info!("line_ver_w{}: {} us", width, start.elapsed().as_micros());
            Timer::after(Duration::from_millis(100)).await;
        }

        // 117-119: Line opacity tests
        let line_opas = [(OPA_COVER, "100"), (OPA_70, "70"), (102, "40")]; // OPA_40 = 102
        for &(opa, name) in line_opas.iter() {
            let start = Instant::now();
            context
                .draw(|surface| {
                    surface.clear(Rgba8888::rgba(0, 0, 0, 255));
                    let mut dsc = LineDsc::new(Point::new(15, 62), Point::new(87, 62));
                    dsc.width = 3;
                    dsc.color = Rgba8888::WHITE;
                    dsc.opa = opa;
                    draw_line(surface, &dsc);
                })
                .await;
            info!("line_opa{}: {} us", name, start.elapsed().as_micros());
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}
