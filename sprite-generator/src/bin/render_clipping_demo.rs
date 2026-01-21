#[path = "../canvas.rs"]
mod canvas;
#[path = "../bmp.rs"]
mod bmp;

use canvas::Canvas;
use rust_gfx::color::Rgba8888;
use rust_gfx::fluent::{
    Axis, FillBuilder, FillPlan, FontHandle, GradientBuilder, GradientStop, Label, LabelAlignment,
    LabelContentPlan, LabelOpacity, LabelSpacing, Line, LineCaps, Radius, Rect, StrokePlan,
    Triangle, Vertices,
};
use rust_gfx::types::{Area, Point, OPA_70};
use std::fs;

fn main() {
    let mut canvas = Canvas::new(200, 150);

    // Base backdrop with a soft vertical gradient.
    let backdrop = GradientBuilder::linear()
        .axis(Axis::Vertical)
        .stops([
            GradientStop::new(0.0, Rgba8888::rgb(22, 28, 45)),
            GradientStop::new(1.0, Rgba8888::rgb(8, 12, 20)),
        ])
        .finish();

    Rect::new()
        .area(Area::new(0, 0, canvas.width as i32 - 1, canvas.height as i32 - 1))
        .fill(FillPlan::Gradient {
            gradient: backdrop.clone(),
        })
        .finish()
        .draw(&mut canvas);

    let clip_window = Area::new(40, 35, 160, 115);

    // Tint the clipped region to make the boundary obvious.
    Rect::new()
        .area(clip_window)
        .fill(
            FillBuilder::solid(Rgba8888::rgb(45, 72, 120))
                .opacity(OPA_70)
                .finish(),
        )
        .finish()
        .draw(&mut canvas);

    // A wide gradient rectangle that visibly trims at the clip boundary.
    let sweep = GradientBuilder::linear()
        .axis(Axis::Horizontal)
        .stops([
            GradientStop::new(0.0, Rgba8888::rgb(255, 96, 64)),
            GradientStop::new(1.0, Rgba8888::rgb(64, 180, 255)),
        ])
        .finish();

    Rect::new()
        .area(Area::new(10, 20, 190, 130))
        .radius(Radius::uniform(28))
        .fill(FillPlan::Gradient {
            gradient: sweep.clone(),
        })
        .clip(clip_window)
        .finish()
        .draw(&mut canvas);

    // Additional geometry that bleeds outside the window to highlight clipping.
    Triangle::new()
        .vertices(Vertices::new3(
            Point::new(55, 25),
            Point::new(182, 60),
            Point::new(120, 140),
        ))
        .fill(
            FillBuilder::solid(Rgba8888::rgb(80, 220, 200))
                .opacity(OPA_70)
                .finish(),
        )
        .clip(clip_window)
        .finish()
        .draw(&mut canvas);

    Line::new()
        .vertices(Vertices::new(Point::new(20, 40), Point::new(180, 110)))
        .stroke(StrokePlan::solid(6, Rgba8888::rgb(18, 200, 235)))
        .caps(LineCaps::round())
        .clip(clip_window)
        .finish()
        .draw(&mut canvas);

    Label::new()
        .origin(Point::new(22, 92))
        .content(LabelContentPlan::text(
            FontHandle::named("montserrat", 18),
            "Clipped content stays inside the frame.",
        ))
        .alignment(LabelAlignment::Start)
        .spacing(LabelSpacing::new(1, 0))
        .opacity(LabelOpacity::new(235))
        .color(Rgba8888::rgb(255, 255, 255))
        .clip(clip_window)
        .finish()
        .draw(&mut canvas);

    // Frame the clipping window for context.
    Rect::new()
        .area(clip_window)
        .stroke(StrokePlan::solid(3, Rgba8888::rgb(255, 255, 255)))
        .finish()
        .draw(&mut canvas);

    Label::new()
        .origin(Point::new(46, 28))
        .content(LabelContentPlan::text(
            FontHandle::named("montserrat", 14),
            "Fluent clip demo",
        ))
        .color(Rgba8888::rgb(210, 220, 240))
        .finish()
        .draw(&mut canvas);

    fs::create_dir_all("sprites").expect("failed to create sprites directory");
    bmp::save_bmp(&canvas, "sprites/clipping_demo.bmp").expect("failed to write BMP");
}
