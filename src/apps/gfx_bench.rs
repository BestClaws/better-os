#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Arc, Circle, RoundedRect};
use crate::libs::gfx::shapes::{Line, Text, Shape};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::rasterizer::Rasterizer;

#[derive(Debug, Clone, Copy)]
pub enum ShapeType {
    RoundedRect,
    Circle,
    Line,
    Arc,
    Text,
}

#[derive(Debug, Clone, Copy)]
pub enum FillType {
    Solid,
    LinearH,
    LinearV,
    Radial,
}

#[derive(Debug, Clone, Copy)]
pub enum StrokeType {
    Thin,   // 1px
    Medium, // 3px
    Thick,  // 6px
}

#[derive(Debug, Clone, Copy)]
pub enum CornerType {
    Sharp,
    Small,   // 2px
    Medium,  // 8px
    Large,   // 20px
    Asymmetric,
}

#[derive(Debug, Clone, Copy)]
pub enum AlphaType {
    Opaque,         // 255
    SemiTransparent, // 128
    LowAlpha,       // 64
    VeryLowAlpha,   // 32
}

#[derive(Debug, Clone, Copy)]
pub struct TestConfig {
    shape_type: ShapeType,
    fill_type: Option<FillType>,
    stroke_type: Option<StrokeType>,
    corner_type: Option<CornerType>,
    alpha_type: AlphaType,
}

fn alpha_value(alpha: AlphaType) -> u8 {
    match alpha {
        AlphaType::Opaque => 255,
        AlphaType::SemiTransparent => 128,
        AlphaType::LowAlpha => 64,
        AlphaType::VeryLowAlpha => 32,
    }
}

fn shape_name(shape: ShapeType) -> &'static str {
    match shape {
        ShapeType::RoundedRect => "RoundedRect",
        ShapeType::Circle => "Circle",
        ShapeType::Line => "Line",
        ShapeType::Arc => "Arc",
        ShapeType::Text => "Text",
    }
}

fn fill_name(fill: Option<FillType>) -> &'static str {
    match fill {
        None => "NoFill",
        Some(FillType::Solid) => "Solid",
        Some(FillType::LinearH) => "LinearH",
        Some(FillType::LinearV) => "LinearV",
        Some(FillType::Radial) => "Radial",
    }
}

fn stroke_name(stroke: Option<StrokeType>) -> &'static str {
    match stroke {
        None => "NoStroke",
        Some(StrokeType::Thin) => "Thin",
        Some(StrokeType::Medium) => "Medium",
        Some(StrokeType::Thick) => "Thick",
    }
}

fn corner_name(corner: Option<CornerType>) -> &'static str {
    match corner {
        None => "NoCorner",
        Some(CornerType::Sharp) => "Sharp",
        Some(CornerType::Small) => "Small",
        Some(CornerType::Medium) => "Medium",
        Some(CornerType::Large) => "Large",
        Some(CornerType::Asymmetric) => "Asym",
    }
}

fn alpha_name(alpha: AlphaType) -> &'static str {
    match alpha {
        AlphaType::Opaque => "Opaque",
        AlphaType::SemiTransparent => "Semi",
        AlphaType::LowAlpha => "Low",
        AlphaType::VeryLowAlpha => "VeryLow",
    }
}

fn generate_all_test_permutations() -> Vec<TestConfig> {
    let mut configs = Vec::new();

    let shapes = [
        ShapeType::RoundedRect,
        // ShapeType::Circle,
        // ShapeType::Line,
        // ShapeType::Arc,
        // ShapeType::Text,
    ];

    let fills = [
        None,
        Some(FillType::Solid),
        Some(FillType::LinearH),
        Some(FillType::LinearV),
        Some(FillType::Radial),
    ];

    let strokes = [
        None,
        Some(StrokeType::Thin),
        Some(StrokeType::Medium),
    ];

    let corners = [
        None,
        Some(CornerType::Large),
        Some(CornerType::Asymmetric),
    ];

    let alphas = [
        AlphaType::Opaque,
        AlphaType::SemiTransparent,
    ];

    for &shape in &shapes {
        for &fill in &fills {
            for &stroke in &strokes {
                for &corner in &corners {
                    for &alpha in &alphas {
                        if matches!(shape, ShapeType::Line | ShapeType::Arc) && fill.is_some() { continue; }
                        if matches!(shape, ShapeType::Text) && (fill.is_some() || stroke.is_some()) { continue; }
                        if !matches!(shape, ShapeType::RoundedRect) && corner.is_some() { continue; }
                        if fill.is_none() && stroke.is_none() && !matches!(shape, ShapeType::Text) { continue; }

                        configs.push(TestConfig { shape_type: shape, fill_type: fill, stroke_type: stroke, corner_type: corner, alpha_type: alpha });
                    }
                }
            }
        }
    }

    configs
}

fn execute_test(surface: &mut DrawingSurface, config: TestConfig) {
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let half_w = (w as f32 * 0.5) as i32;
    let half_h = (h as f32 * 0.5) as i32;
    let left = (w / 2) - half_w / 2;
    let top = (h / 2) - half_h / 2;
    let alpha = alpha_value(config.alpha_type);

    match config.shape_type {
        ShapeType::RoundedRect => {
            let (tl, tr, bl, br) = match config.corner_type.unwrap_or(CornerType::Sharp) {
                CornerType::Sharp => (0, 0, 0, 0),
                CornerType::Small => (2, 2, 2, 2),
                CornerType::Medium => (8, 8, 8, 8),
                CornerType::Large => (20, 20, 20, 20),
                CornerType::Asymmetric => (12, 4, 16, 0),
            };
            let mut rr = RoundedRect::new(left, top, half_w, half_h, tl, tr, bl, br);
            if let Some(stroke) = config.stroke_type { rr = rr.stroke(match stroke { StrokeType::Thin => 1, StrokeType::Medium => 3, StrokeType::Thick => 6 }, Rgba8888::rgba(255,255,255,alpha)); }
            if let Some(fill) = config.fill_type {
                rr = match fill {
                    FillType::Solid => rr.fill_solid(Rgba8888::rgba(30,30,30,alpha)),
                    FillType::LinearH => rr.fill_linear_h(Rgba8888::rgba(255,0,0,alpha), Rgba8888::rgba(0,0,255,alpha)),
                    FillType::LinearV => rr.fill_linear_v(Rgba8888::rgba(0,255,0,alpha), Rgba8888::rgba(0,0,255,alpha)),
                    FillType::Radial => rr.fill_radial(Rgba8888::rgba(255,255,255,alpha), Rgba8888::rgba(30,30,30,alpha)),
                };
            }
            rr.draw(surface);
        }
        ShapeType::Circle => {
            let radius = (half_w.min(half_h) / 2).max(4);
            let mut circle = Circle::new(w/2, h/2, radius);
            if let Some(stroke) = config.stroke_type { circle = circle.stroke(match stroke { StrokeType::Thin => 1, StrokeType::Medium => 3, StrokeType::Thick => 6 }, Rgba8888::rgba(255,255,255,alpha)); }
            if let Some(fill) = config.fill_type {
                circle = match fill {
                    FillType::Solid => circle.fill_solid(Rgba8888::rgba(200, 120, 40, alpha)),
                    FillType::LinearH => circle.fill_linear_h(Rgba8888::rgba(255,0,0,alpha), Rgba8888::rgba(0,0,255,alpha)),
                    FillType::LinearV => circle.fill_linear_v(Rgba8888::rgba(0,255,0,alpha), Rgba8888::rgba(0,0,255,alpha)),
                    FillType::Radial => circle.fill_radial(Rgba8888::rgba(255,255,255,alpha), Rgba8888::rgba(30,30,30,alpha)),
                };
            }
            circle.draw(surface);
        }
        ShapeType::Line => {
            let mut line = Line::new(left, h/2, left + half_w, h/2);
            let stroke = config.stroke_type.unwrap_or(StrokeType::Thin);
            line = line.stroke(match stroke { StrokeType::Thin => 1, StrokeType::Medium => 3, StrokeType::Thick => 6 }, Rgba8888::rgba(255,255,255,alpha));
            line.draw(surface);
        }
        ShapeType::Arc => {
            let radius = (half_w.min(half_h) / 2).max(4);
            let mut arc = Arc::new(w/2, h/2, radius, 0, 180);
            let stroke = config.stroke_type.unwrap_or(StrokeType::Thin);
            arc = arc.stroke(match stroke { StrokeType::Thin => 1, StrokeType::Medium => 3, StrokeType::Thick => 6 }, Rgba8888::rgba(255,255,255,alpha));
            arc.draw(surface);
        }
        ShapeType::Text => {
            let mut text = Text::new(w/2 - 24, h/2 + 10, "12:34");
            text = text.color(Rgba8888::rgba(255,255,255,alpha));
            text.draw(surface);
        }
    }
}

#[embassy_executor::task]
pub async fn gfx_bench_app(context: AppContext) {
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        let configs = generate_all_test_permutations();
        info!("generated {} test configs", configs.len());
        for cfg in configs.iter() {
            info!("test: {} {} {} {} {}",
                shape_name(cfg.shape_type),
                fill_name(cfg.fill_type),
                stroke_name(cfg.stroke_type),
                corner_name(cfg.corner_type),
                alpha_name(cfg.alpha_type)
            );
            context.draw(|surface: &mut DrawingSurface| {
                surface.fill_rect(0, 0, surface.width() as i32, surface.height() as i32, Rgba8888::rgba(0,0,0,255));
                let t0 = Instant::now();
                execute_test(surface, *cfg);
                let us = (Instant::now() - t0).as_micros();
                info!("bench {}: {} us", shape_name(cfg.shape_type), us);
            }).await;
            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}
