use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::{Rasterizer, Circle, RoundedRect, Arc};
use defmt::info;
use embassy_time::{Duration, Instant, Timer};
use crate::util::math::primitives::Point;

/// Simplified Graphics Performance Benchmark (migrated to new gfx API)
#[embassy_executor::task]
pub async fn gfx_perf_bench_adaptive_app(ctx: AppContext) {
    info!("Starting simplified GFX benchmark");
    Timer::after(Duration::from_millis(1)).await;

    for _step in 0..50 {
        let draw_start = Instant::now();
        ctx.draw(|surface: &mut DrawingSurface| {
            let raster: &mut dyn Rasterizer = surface;
            let w = raster.width() as i32;
            let h = raster.height() as i32;
            raster.fill_rect(0, 0, w, h, Rgba8888::rgba(12, 14, 22, 255));

            // Draw circles
            for i in 0..6 {
                let cx = (w / 7) * (i + 1);
                Circle::new(Point::new(cx, h / 2), 18)
                    .fill_solid(Rgba8888::rgba(80 + (i as u8)*20, 120, 200, 200))
                    .draw(raster);
            }

            // Draw rounded rects
            for i in 0..3 {
                let x = 10 + i * 60;
                RoundedRect::new(x, 10, 50, 30, 6, 6, 6, 6)
                    .fill_linear_h(Rgba8888::rgba(200, 80, 80, 180), Rgba8888::rgba(120, 180, 255, 200))
                    .draw(raster);
            }

            // Draw arcs
            Arc::new(Point::new(w - 40, h - 40), 28, 0, (core::f32::consts::PI * 1.5) as i32)
                .stroke(3, Rgba8888::rgba(100, 255, 150, 255))
                .draw(raster);
        }).await;

        let t = draw_start.elapsed();
        info!("frame: {}us", t.as_micros());
        Timer::after(Duration::from_millis(20)).await;
    }
}
        // ShapeType::Rectangle,       // Fast rectangles with corner radius support
        ShapeType::Circle,          // Optimized circle fills and strokes  
        ShapeType::Line,            // Lines with Wu/Bresenham algorithms
        ShapeType::Arc,             // Arc rendering with angular stepping
        // ShapeType::BezierQuadratic, // Quadratic bezier curves
        // ShapeType::BezierCubic,     // Cubic bezier curves
    ];

    // ============================================================================
    // FILL TYPE SELECTION - Comment/uncomment lines to select which to benchmark
    // ============================================================================
    let fills = [
        None,                                       // No fill (stroke only)
        Some(FillType::Solid),                     // Solid color fills (fastest)
        Some(FillType::LinearGradient),            // Linear gradients
        Some(FillType::LinearGradientHorizontal),  // Horizontal linear gradients
        Some(FillType::RadialGradient),            // Radial gradients
        Some(FillType::RadialGradientOffCenter),   // Off-center radial gradients
    ];

    // ============================================================================
    // STROKE TYPE SELECTION - Comment/uncomment lines to select which to benchmark
    // ============================================================================
    let strokes = [
        None,                       // No stroke (fill only)
        // Some(StrokeType::Thin),     // 1.0px stroke width
        Some(StrokeType::Medium),   // 3.0px stroke width  
        // Some(StrokeType::Thick),    // 6.0px stroke width
    ];

    // ============================================================================
    // ANTI-ALIASING SELECTION - Comment/uncomment lines to select which to benchmark
    // ============================================================================
    let aa_types = [
        None,                           // No anti-aliasing (fastest)
        Some(AntiAliasing::Low),        // Low quality AA
        // Some(AntiAliasing::Medium),     // Medium quality AA
        // Some(AntiAliasing::High),       // High quality AA (slowest)
    ];

    // ============================================================================
    // CORNER RADIUS SELECTION - Comment/uncomment lines to select which to benchmark
    // NOTE: Corner radii only apply to rectangles! If no rectangles selected, use None only
    // ============================================================================
    let corners = [
        None,                           // No corner radius (rectangles only)
        // Some(CornerType::Sharp),        // Sharp corners (0px radius)
        // Some(CornerType::Small),        // Small radius (2px)
        // Some(CornerType::Medium),       // Medium radius (8px)
        // Some(CornerType::Large),        // Large radius (20px)
        // Some(CornerType::Asymmetric),   // Different radius per corner
    ];

    // ============================================================================
    // ALPHA TRANSPARENCY SELECTION - Comment/uncomment lines to select which to benchmark
    // ============================================================================
    let alphas = [
        AlphaType::Opaque,          // 255 alpha - fastest path (no blending)
        AlphaType::SemiTransparent, // 128 alpha - blending required
        AlphaType::LowAlpha,        // 64 alpha - heavy blending
        AlphaType::VeryLowAlpha,    // 32 alpha - very heavy blending
    ];

    // Generate all valid combinations
    for &shape in &shapes {
        for &fill in &fills {
            for &stroke in &strokes {
                for &aa in &aa_types {
                    for &corner in &corners {
                        for &alpha in &alphas {
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
                                alpha_type: alpha,
                            };

                            if configs.push(config).is_err() {
                                warn!("⚠️  Reached maximum test configurations (2048)");
                                return configs;
                            }
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
                rect = rect.fill(create_paint(fill_type, left, top, test_width, test_height, config.alpha_type));
            }

            // Apply stroke
            if let Some(stroke_type) = config.stroke_type {
                rect = rect.stroke(create_stroke(stroke_type, config.alpha_type));
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
                circle = circle.fill(create_paint(fill_type, center_x - radius as i32, center_y - radius as i32, (radius * 2.0) as i32, (radius * 2.0) as i32, config.alpha_type));
            }

            // Apply stroke
            if let Some(stroke_type) = config.stroke_type {
                circle = circle.stroke(create_stroke(stroke_type, config.alpha_type));
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
                line = line.stroke(create_stroke(stroke_type, config.alpha_type));
            } else {
                line = line.stroke(create_stroke(StrokeType::Thin, config.alpha_type)); // Default stroke
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
                arc = arc.stroke(create_stroke(stroke_type, config.alpha_type));
            } else {
                arc = arc.stroke(create_stroke(StrokeType::Thin, config.alpha_type)); // Default stroke
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
                bezier = bezier.stroke(create_stroke(stroke_type, config.alpha_type));
            } else {
                bezier = bezier.stroke(create_stroke(StrokeType::Thin, config.alpha_type)); // Default stroke
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
                bezier = bezier.stroke(create_stroke(stroke_type, config.alpha_type));
            } else {
                bezier = bezier.stroke(create_stroke(StrokeType::Thin, config.alpha_type)); // Default stroke
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
fn create_paint(fill_type: FillType, x: i32, y: i32, width: i32, height: i32, alpha_type: AlphaType) -> Paint {
    let alpha = match alpha_type {
        AlphaType::Opaque => 255,
        AlphaType::SemiTransparent => 128,
        AlphaType::LowAlpha => 64,
        AlphaType::VeryLowAlpha => 32,
    };

    match fill_type {
        FillType::Solid => Paint::solid(Rgba8888::new(100, 150, 255, alpha)),
        FillType::LinearGradient => Paint::linear(
            Point::new(x, y),
            Point::new(x + width, y + height),
            Rgba8888::new(255, 100, 100, alpha),
            Rgba8888::new(100, 100, 255, alpha)
        ),
        FillType::LinearGradientHorizontal => Paint::linear(
            Point::new(x, y + height / 2),
            Point::new(x + width, y + height / 2),
            Rgba8888::new(255, 200, 100, alpha),
            Rgba8888::new(100, 200, 255, alpha)
        ),
        FillType::RadialGradient => Paint::radial(
            Point::new(x + width / 2, y + height / 2),
            (width.min(height) / 2) as f32,
            Rgba8888::new(255, 255, 100, alpha),
            Rgba8888::new(255, 100, 100, alpha.saturating_sub(50))
        ),
        FillType::RadialGradientOffCenter => Paint::radial(
            Point::new(x + width / 4, y + height / 4),
            (width.min(height) * 3 / 4) as f32,
            Rgba8888::new(100, 255, 255, alpha),
            Rgba8888::new(255, 100, 255, alpha.saturating_sub(30))
        ),
    }
}

/// Create stroke based on stroke type and alpha
fn create_stroke(stroke_type: StrokeType, alpha_type: AlphaType) -> Stroke {
    let alpha = match alpha_type {
        AlphaType::Opaque => 255,
        AlphaType::SemiTransparent => 128,
        AlphaType::LowAlpha => 64,
        AlphaType::VeryLowAlpha => 32,
    };

    let (width, color) = match stroke_type {
        StrokeType::Thin => (1.0, Rgba8888::new(255, 255, 255, alpha)),
        StrokeType::Medium => (3.0, Rgba8888::new(255, 255, 0, alpha)),
        StrokeType::Thick => (6.0, Rgba8888::new(255, 0, 255, alpha)),
    };
    Stroke::new(color, width)
}

/// Create corner radii based on corner type
fn create_corner_radii(corner_type: CornerType) -> CornerRadii {
    match corner_type {
        CornerType::Sharp => CornerRadii::uniform(FixedI32::<U16>::from_num(0.0)),
        CornerType::Small => CornerRadii::uniform(FixedI32::<U16>::from_num(2.0)),
        CornerType::Medium => CornerRadii::uniform(FixedI32::<U16>::from_num(8.0)),
        CornerType::Large => CornerRadii::uniform(FixedI32::<U16>::from_num(20.0)),
        CornerType::Asymmetric => CornerRadii::new(
            12.0, // top_left
            4.0,  // top_right
            16.0, // bottom_right
            0.0,  // bottom_left
        ),
    }
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
        Some(FillType::LinearGradientHorizontal) => "LinearHGrad",
        Some(FillType::RadialGradient) => "RadialGrad",
        Some(FillType::RadialGradientOffCenter) => "RadialOGrad",
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
        Some(CornerType::Asymmetric) => "AsymCorner",
    }
}

/// Convert alpha type to string for logging
fn alpha_name(alpha: AlphaType) -> &'static str {
    match alpha {
        AlphaType::Opaque => "Opaque",
        AlphaType::SemiTransparent => "SemiAlpha",
        AlphaType::LowAlpha => "LowAlpha",
        AlphaType::VeryLowAlpha => "VeryLowAlpha",
    }
}
