extern crate alloc;
use crate::libs::gfx::{Color, Layer, Opacity, Point, Rect};
use crate::libs::gfx::primitives::{Arc, Border, BorderSide, Circle, Fill, Line, Triangle};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use alloc::vec::Vec;
use defmt::info;
use embassy_time::{Duration, Instant, Timer};

#[derive(Debug, Clone, Copy)]
pub enum ShapeType {
    Rect,
    Circle,
    Line,
    Arc,
    Border,
    Triangle,
}

#[derive(Debug, Clone, Copy)]
pub enum FillType {
    Solid,
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
    Small,  // 2px
    Medium, // 8px
    Large,  // 20px
}

#[derive(Debug, Clone, Copy)]
pub enum AlphaType {
    Opaque,          // 255
    SemiTransparent, // 128
    LowAlpha,        // 64
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
    }
}

fn shape_name(shape: ShapeType) -> &'static str {
    match shape {
        ShapeType::Rect => "Rect",
        ShapeType::Circle => "Circle",
        ShapeType::Line => "Line",
        ShapeType::Arc => "Arc",
        ShapeType::Border => "Border",
        ShapeType::Triangle => "Triangle",
    }
}

fn fill_name(fill: Option<FillType>) -> &'static str {
    match fill {
        None => "NoFill",
        Some(FillType::Solid) => "Solid",
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
    }
}

fn alpha_name(alpha: AlphaType) -> &'static str {
    match alpha {
        AlphaType::Opaque => "Opaque",
        AlphaType::SemiTransparent => "Semi",
        AlphaType::LowAlpha => "Low",
    }
}

fn generate_all_test_permutations() -> Vec<TestConfig> {
    let mut configs = Vec::new();

    let shapes = [
        ShapeType::Rect,
        ShapeType::Circle,
        ShapeType::Line,
        ShapeType::Arc,
        ShapeType::Border,
        ShapeType::Triangle,
    ];

    let fills = [
        None,
        Some(FillType::Solid),
    ];

    let strokes = [None, Some(StrokeType::Thin), Some(StrokeType::Medium)];

    let corners = [None, Some(CornerType::Large)];

    let alphas = [AlphaType::Opaque, AlphaType::SemiTransparent];

    for &shape in &shapes {
        for &fill in &fills {
            for &stroke in &strokes {
                for &corner in &corners {
                    for &alpha in &alphas {
                        // Skip invalid combinations
                        if matches!(shape, ShapeType::Line | ShapeType::Arc) && fill.is_some() {
                            continue;
                        }
                        if matches!(shape, ShapeType::Border) && fill.is_some() {
                            continue;
                        }
                        if !matches!(shape, ShapeType::Rect) && corner.is_some() {
                            continue;
                        }
                        if fill.is_none() && stroke.is_none() {
                            continue;
                        }

                        configs.push(TestConfig {
                            shape_type: shape,
                            fill_type: fill,
                            stroke_type: stroke,
                            corner_type: corner,
                            alpha_type: alpha,
                        });
                    }
                }
            }
        }
    }

    configs
}

fn execute_test(surface: &mut DrawingSurface, config: TestConfig) {
    let mut layer = Layer::from_draw_target(surface);
    
    let w = layer.width() as i32;
    let h = layer.height() as i32;
    let half_w = w / 2;
    let half_h = h / 2;
    let left = (w - half_w) / 2;
    let top = (h - half_h) / 2;
    let opacity = Opacity::new(alpha_value(config.alpha_type));

    match config.shape_type {
        ShapeType::Rect => {
            let radius = match config.corner_type.unwrap_or(CornerType::Sharp) {
                CornerType::Sharp => 0,
                CornerType::Small => 2,
                CornerType::Medium => 8,
                CornerType::Large => 20,
            };
            
            if config.fill_type.is_some() {
                Fill::new(&mut layer, Rect::new(left, top, half_w as u32, half_h as u32))
                    .color(Color::rgb(30, 30, 100))
                    .opacity(opacity)
                    .radius(radius)
                    .draw();
            }
            
            if let Some(stroke) = config.stroke_type {
                let width = match stroke {
                    StrokeType::Thin => 1,
                    StrokeType::Medium => 3,
                    StrokeType::Thick => 6,
                };
                Border::new(&mut layer, Rect::new(left, top, half_w as u32, half_h as u32))
                    .color(Color::WHITE)
                    .width(width)
                    .opacity(opacity)
                    .radius(radius)
                    .draw();
            }
        }
        ShapeType::Circle => {
            let radius = (half_w.min(half_h) / 2).max(4) as u16;
            
            // Choose between fill or stroke (not both in this benchmark)
            if config.fill_type.is_some() {
                Circle::new(&mut layer, Point::new(w / 2, h / 2), radius)
                    .color(Color::rgb(200, 120, 40))
                    .opacity(opacity)
                    .draw();
            } else if let Some(stroke) = config.stroke_type {
                let width = match stroke {
                    StrokeType::Thin => 1,
                    StrokeType::Medium => 3,
                    StrokeType::Thick => 6,
                };
                Arc::new(&mut layer, Point::new(w / 2, h / 2), radius)
                    .angles(0, 360)
                    .color(Color::WHITE)
                    .width(width)
                    .opacity(opacity)
                    .draw();
            }
        }
        ShapeType::Line => {
            let width = match config.stroke_type.unwrap_or(StrokeType::Thin) {
                StrokeType::Thin => 1,
                StrokeType::Medium => 3,
                StrokeType::Thick => 6,
            };
            Line::new(
                &mut layer,
                crate::libs::gfx::core::geometry::PointF::new(left as f32, (h / 2) as f32),
                crate::libs::gfx::core::geometry::PointF::new((left + half_w) as f32, (h / 2) as f32),
            )
            .color(Color::WHITE)
            .width(width)
            .opacity(opacity)
            .draw();
        }
        ShapeType::Arc => {
            let radius = (half_w.min(half_h) / 2).max(4) as u16;
            let width = match config.stroke_type.unwrap_or(StrokeType::Thin) {
                StrokeType::Thin => 1,
                StrokeType::Medium => 3,
                StrokeType::Thick => 6,
            };
            Arc::new(&mut layer, Point::new(w / 2, h / 2), radius)
                .angles(0, 180)
                .color(Color::WHITE)
                .width(width)
                .opacity(opacity)
                .draw();
        }
        ShapeType::Border => {
            let width = match config.stroke_type.unwrap_or(StrokeType::Thin) {
                StrokeType::Thin => 1,
                StrokeType::Medium => 3,
                StrokeType::Thick => 6,
            };
            Border::new(&mut layer, Rect::new(left, top, half_w as u32, half_h as u32))
                .color(Color::WHITE)
                .width(width)
                .opacity(opacity)
                .draw();
        }
        ShapeType::Triangle => {
            use crate::libs::gfx::core::geometry::PointF;
            
            let p1 = PointF::new((w / 2) as f32, (top + 20) as f32);
            let p2 = PointF::new((left + half_w - 20) as f32, (top + half_h - 20) as f32);
            let p3 = PointF::new((left + 20) as f32, (top + half_h - 20) as f32);
            
            if config.fill_type.is_some() {
                Triangle::new(&mut layer, p1, p2, p3)
                    .color(Color::rgb(100, 200, 100))
                    .opacity(opacity)
                    .draw();
            }
        }
    }
}

#[embassy_executor::task]
pub async fn gfx_bench_app(context: AppContext) {
    info!("Starting gfx_bench app");

    let configs = generate_all_test_permutations();
    info!("Generated {} test configurations", configs.len());

    let mut next_cfg = 0usize;

    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let cfg = configs[next_cfg];
        next_cfg = (next_cfg + 1) % configs.len();

        let draw_start = Instant::now();
        context
            .draw(|surface: &mut DrawingSurface| {
                // Clear background
                let mut layer = Layer::from_draw_target(surface);
                layer.clear(Color::BLACK);
                
                // Run test
                execute_test(surface, cfg);
            })
            .await;
        let elapsed = draw_start.elapsed();

        info!(
            "bench {} {} {} {} {}: {} us",
            shape_name(cfg.shape_type),
            fill_name(cfg.fill_type),
            stroke_name(cfg.stroke_type),
            corner_name(cfg.corner_type),
            alpha_name(cfg.alpha_type),
            elapsed.as_micros()
        );

        Timer::after(Duration::from_millis(20)).await;
    }
}
