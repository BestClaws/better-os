/// Comprehensive Graphics Performance Benchmarking App
///
/// Tests all permutations of shapes, fills, strokes, and anti-aliasing variants
/// Each test draws centered at ~50% screen size with 500ms delays

use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::two_d::{
    Canvas2D, Rasterizer, Point, Size, Rgba8888, Paint, Stroke, Drawable,
    PrimitiveRect as Rect, Circle, Line, Arc, Bezier, CornerRadii, AntiAliasing, FixedI32, U16
};
use crate::libs::gfx::two_d::paint::{LinearGradient, RadialGradient};
use crate::libs::gfx::two_d::stroke::{LineCap, LineJoin};
use defmt::{info, warn, error};
use embassy_time::{Instant, Duration, Timer};

/// Test configuration for comprehensive benchmarking
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct TestConfig {
    shape_type: ShapeType,
    fill_type: Option<FillType>,
    stroke_type: Option<StrokeType>,
    aa_type: Option<AntiAliasing>,
    corner_type: Option<CornerType>,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum ShapeType {
    Rectangle,
    Circle,
    Line,
    Arc,
    BezierQuadratic,
    BezierCubic,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum FillType {
    Solid,
    LinearGradient,
    RadialGradient,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum StrokeType {
    Thin,      // 1.0px
    Medium,    // 3.0px
    Thick,     // 6.0px
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum CornerType {
    Sharp,
    Small,     // 2.0px radius
    Medium,    // 8.0px radius
    Large,     // 20.0px radius
}

/// Comprehensive performance benchmarking application
#[embassy_executor::task]
pub async fn gfx_perf_bench_adaptive_app(ctx: AppContext) {
    info!("🚀 Starting Comprehensive Graphics Performance Benchmark");

    // Wait for app initialization
    Timer::after(Duration::from_millis(50)).await;

    // Generate all test permutations
    let test_configs = generate_all_test_permutations();
    info!("📊 Generated {} comprehensive test configurations", test_configs.len());

    let mut total_render_time = Duration::from_ticks(0);
    let mut test_count = 0;
    let benchmark_start = Instant::now();

    // Execute each test with 500ms delay
    for (i, config) in test_configs.iter().enumerate() {
        info!("🧪 Test {}/{}: {} {} {} {} {}",
              i + 1, test_configs.len(),
              shape_name(config.shape_type),
              fill_name(config.fill_type),
              stroke_name(config.stroke_type),
              aa_name(config.aa_type),
              corner_name(config.corner_type));

        let draw_start = Instant::now();

        ctx.draw(|surface: &mut DrawingSurface| {
            // Create Canvas2D
            let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
            let canvas_width = canvas.width() as i32;
            let canvas_height = canvas.height() as i32;

            // Clear canvas
            canvas.clear_color(Rgba8888::new(20, 25, 35, 255));

            // Execute the specific test
            execute_test(&mut canvas, *config, canvas_width, canvas_height);
        }).await;

        let test_time = draw_start.elapsed();
        info!("  ⏱️  Render time: {}μs", test_time.as_micros());
        total_render_time += test_time;
        test_count += 1;

        // Wait 500ms before next test
        Timer::after(Duration::from_millis(500)).await;
    }

    // Final comprehensive statistics
    let total_benchmark_time = benchmark_start.elapsed();
    let avg_render_time = if test_count > 0 { total_render_time / test_count as u32 } else { Duration::from_ticks(0) };

    info!("📊 COMPREHENSIVE PERFORMANCE SUMMARY:");
    info!("   Total test configurations: {}", test_count);
    info!("   Total render time: {}μs", total_render_time.as_micros());
    info!("   Total benchmark time: {}μs", total_benchmark_time.as_micros());
    info!("   Average per test: {}μs", avg_render_time.as_micros());

    // Performance analysis
    let avg_micros = avg_render_time.as_micros();
    if avg_micros < 1000 {
        info!("   ✅ Excellent performance: {}μs average render time", avg_micros);
    } else if avg_micros < 5000 {
        info!("   ✅ Good performance: {}μs average render time", avg_micros);
    } else if avg_micros < 10000 {
        info!("   ⚠️  Moderate performance: {}μs average render time", avg_micros);
    } else {
        info!("   🔥 Performance needs improvement: {}μs average render time", avg_micros);
    }

    let throughput = test_count as f32 / (total_render_time.as_micros() as f32 / 1_000_000.0);
    info!("   🎯 Render throughput: {} ops/sec", throughput as u32);

    info!("🏁 Comprehensive Graphics Performance Benchmark Complete");
}

/// Generate all possible test permutations for comprehensive benchmarking
fn generate_all_test_permutations() -> heapless::Vec<TestConfig, 256> {
    let mut configs = heapless::Vec::new();

    let shapes = [
        ShapeType::Rectangle,
        ShapeType::Circle,
        ShapeType::Line,
        ShapeType::Arc,
        ShapeType::BezierQuadratic,
        ShapeType::BezierCubic,
    ];

    let fills = [
        None,
        Some(FillType::Solid),
        Some(FillType::LinearGradient),
        Some(FillType::RadialGradient),
    ];

    let strokes = [
        None,
        Some(StrokeType::Thin),
        Some(StrokeType::Medium),
        Some(StrokeType::Thick),
    ];

    let aa_types = [
        None,
        Some(AntiAliasing::Low),
        Some(AntiAliasing::Medium),
        Some(AntiAliasing::High),
    ];

    let corners = [
        None,
        Some(CornerType::Sharp),
        Some(CornerType::Small),
        Some(CornerType::Medium),
        Some(CornerType::Large),
    ];

    // Generate all valid combinations
    for &shape in &shapes {
        for &fill in &fills {
            for &stroke in &strokes {
                for &aa in &aa_types {
                    for &corner in &corners {
                        // Skip invalid combinations
                        if !is_valid_combination(shape, fill, stroke, corner) {
                            continue;
                        }

                        // Skip if no fill and no stroke (invisible)
                        if fill.is_none() && stroke.is_none() {
                            continue;
                        }

                        let config = TestConfig {
                            shape_type: shape,
                            fill_type: fill,
                            stroke_type: stroke,
                            aa_type: aa,
                            corner_type: corner,
                        };

                        if configs.push(config).is_err() {
                            warn!("⚠️  Reached maximum test configurations (256)");
                            return configs;
                        }
                    }
                }
            }
        }
    }

    configs
}

/// Check if a combination of parameters is valid
fn is_valid_combination(shape: ShapeType, fill: Option<FillType>, stroke: Option<StrokeType>, corner: Option<CornerType>) -> bool {
    match shape {
        ShapeType::Rectangle => true, // Rectangles support all combinations
        ShapeType::Circle => corner.is_none(), // Circles don't have corners
        ShapeType::Line => fill.is_none() && corner.is_none(), // Lines only have strokes
        ShapeType::Arc => fill.is_none() && corner.is_none(), // Arcs only have strokes
        ShapeType::BezierQuadratic | ShapeType::BezierCubic => fill.is_none() && corner.is_none(), // Beziers only have strokes
    }
}

/// Execute a specific test configuration
fn execute_test(canvas: &mut Canvas2D, config: TestConfig, width: i32, height: i32) {
    // Calculate centered position with ~50% screen dimensions
    let test_width = (width as f32 * 0.5) as i32;
    let test_height = (height as f32 * 0.5) as i32;
    let center_x = width / 2;
    let center_y = height / 2;
    let left = center_x - test_width / 2;
    let top = center_y - test_height / 2;

    match config.shape_type {
        ShapeType::Rectangle => {
            let mut rect = Rect::from_coords(left, top, test_width as u32, test_height as u32);

            // Apply fill
            if let Some(fill_type) = config.fill_type {
                rect = rect.fill(create_paint(fill_type, left, top, test_width, test_height));
            }

            // Apply stroke
            if let Some(stroke_type) = config.stroke_type {
                rect = rect.stroke(create_stroke(stroke_type));
            }

            // Apply corners
            if let Some(corner_type) = config.corner_type {
                rect = rect.corner_radii(create_corner_radii(corner_type));
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                rect = rect.aa(aa);
            }

            rect.draw(canvas);
        },

        ShapeType::Circle => {
            let radius = (test_width.min(test_height) / 2) as f32;
            let mut circle = Circle::new(Point::new(center_x, center_y), radius);

            // Apply fill
            if let Some(fill_type) = config.fill_type {
                circle = circle.fill(create_paint(fill_type, center_x - radius as i32, center_y - radius as i32, (radius * 2.0) as i32, (radius * 2.0) as i32));
            }

            // Apply stroke
            if let Some(stroke_type) = config.stroke_type {
                circle = circle.stroke(create_stroke(stroke_type));
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                circle = circle.aa(aa);
            }

            circle.draw(canvas);
        },

        ShapeType::Line => {
            let mut line = Line::new(
                Point::new(left, center_y),
                Point::new(left + test_width, center_y)
            );

            // Apply stroke (required for lines)
            if let Some(stroke_type) = config.stroke_type {
                line = line.stroke(create_stroke(stroke_type));
            } else {
                line = line.stroke(create_stroke(StrokeType::Thin)); // Default stroke
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                line = line.aa(aa);
            }

            line.draw(canvas);
        },

        ShapeType::Arc => {
            let radius = (test_width.min(test_height) / 2) as f32;
            let mut arc = Arc::new(
                Point::new(center_x, center_y),
                radius,
                0.0,
                core::f32::consts::PI
            );

            // Apply stroke (required for arcs)
            if let Some(stroke_type) = config.stroke_type {
                arc = arc.stroke(create_stroke(stroke_type));
            } else {
                arc = arc.stroke(create_stroke(StrokeType::Thin)); // Default stroke
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                arc = arc.aa(aa);
            }

            arc.draw(canvas);
        },

        ShapeType::BezierQuadratic => {
            let mut bezier = Bezier::quadratic(
                Point::new(left, center_y),
                Point::new(center_x, top),
                Point::new(left + test_width, center_y)
            );

            // Apply stroke (required for beziers)
            if let Some(stroke_type) = config.stroke_type {
                bezier = bezier.stroke(create_stroke(stroke_type));
            } else {
                bezier = bezier.stroke(create_stroke(StrokeType::Thin)); // Default stroke
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                bezier = bezier.aa(aa);
            }

            bezier.draw(canvas);
        },

        ShapeType::BezierCubic => {
            let mut bezier = Bezier::cubic(
                Point::new(left, center_y),
                Point::new(left + test_width / 3, top),
                Point::new(left + test_width * 2 / 3, top + test_height),
                Point::new(left + test_width, center_y)
            );

            // Apply stroke (required for beziers)
            if let Some(stroke_type) = config.stroke_type {
                bezier = bezier.stroke(create_stroke(stroke_type));
            } else {
                bezier = bezier.stroke(create_stroke(StrokeType::Thin)); // Default stroke
            }

            // Apply anti-aliasing
            if let Some(aa) = config.aa_type {
                bezier = bezier.aa(aa);
            }

            bezier.draw(canvas);
        },
    }
}

/// Create paint based on fill type
fn create_paint(fill_type: FillType, x: i32, y: i32, width: i32, height: i32) -> Paint {
    match fill_type {
        FillType::Solid => Paint::solid(Rgba8888::new(100, 150, 255, 255)),
        FillType::LinearGradient => Paint::linear(
            Point::new(x, y),
            Point::new(x + width, y + height),
            Rgba8888::new(255, 100, 100, 255),
            Rgba8888::new(100, 100, 255, 255)
        ),
        FillType::RadialGradient => Paint::radial(
            Point::new(x + width / 2, y + height / 2),
            (width.min(height) / 2) as f32,
            Rgba8888::new(255, 255, 100, 255),
            Rgba8888::new(255, 100, 100, 100)
        ),
    }
}

/// Create stroke based on stroke type
fn create_stroke(stroke_type: StrokeType) -> Stroke {
    let (width, color) = match stroke_type {
        StrokeType::Thin => (1.0, Rgba8888::new(255, 255, 255, 255)),
        StrokeType::Medium => (3.0, Rgba8888::new(255, 255, 0, 255)),
        StrokeType::Thick => (6.0, Rgba8888::new(255, 0, 255, 255)),
    };
    Stroke::new(color, width)
}

/// Create corner radii based on corner type
fn create_corner_radii(corner_type: CornerType) -> CornerRadii {
    let radius = match corner_type {
        CornerType::Sharp => 0.0,
        CornerType::Small => 2.0,
        CornerType::Medium => 8.0,
        CornerType::Large => 20.0,
    };
    CornerRadii::uniform(FixedI32::<U16>::from_num(radius))
}

/// Convert shape type to string for logging
fn shape_name(shape: ShapeType) -> &'static str {
    match shape {
        ShapeType::Rectangle => "Rect",
        ShapeType::Circle => "Circle",
        ShapeType::Line => "Line",
        ShapeType::Arc => "Arc",
        ShapeType::BezierQuadratic => "BezierQ",
        ShapeType::BezierCubic => "BezierC",
    }
}

/// Convert fill type to string for logging
fn fill_name(fill: Option<FillType>) -> &'static str {
    match fill {
        None => "NoFill",
        Some(FillType::Solid) => "SolidFill",
        Some(FillType::LinearGradient) => "LinearGrad",
        Some(FillType::RadialGradient) => "RadialGrad",
    }
}

/// Convert stroke type to string for logging
fn stroke_name(stroke: Option<StrokeType>) -> &'static str {
    match stroke {
        None => "NoStroke",
        Some(StrokeType::Thin) => "ThinStroke",
        Some(StrokeType::Medium) => "MedStroke",
        Some(StrokeType::Thick) => "ThickStroke",
    }
}

/// Convert AA type to string for logging
fn aa_name(aa: Option<AntiAliasing>) -> &'static str {
    match aa {
        None => "NoAA",
        Some(AntiAliasing::None) => "NoAA",
        Some(AntiAliasing::Low) => "LowAA",
        Some(AntiAliasing::Medium) => "MedAA",
        Some(AntiAliasing::High) => "HighAA",
    }
}

/// Convert corner type to string for logging
fn corner_name(corner: Option<CornerType>) -> &'static str {
    match corner {
        None => "NoCorner",
        Some(CornerType::Sharp) => "SharpCorner",
        Some(CornerType::Small) => "SmallCorner",
        Some(CornerType::Medium) => "MedCorner",
        Some(CornerType::Large) => "LargeCorner",
    }
}
