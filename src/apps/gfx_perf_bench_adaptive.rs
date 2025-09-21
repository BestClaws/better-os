/// Adaptive Graphics Performance Benchmarking App
/// 
/// This app adapts to any canvas size and focuses on the major performance bottlenecks
/// identified from the initial benchmark run.

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

/// Adaptive performance benchmarking application
#[embassy_executor::task]
pub async fn gfx_perf_bench_adaptive_app(ctx: AppContext) {
    info!("🚀 Starting Adaptive Graphics Performance Benchmark");
    
    // Wait a bit to ensure the app is properly initialized
    Timer::after(Duration::from_millis(500)).await;
    
    // Perform all benchmarks in a single draw call
    ctx.draw(|surface: &mut DrawingSurface| {
        let benchmark_start = Instant::now();
        
        // Create Canvas2D
        let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
        let canvas_width = canvas.width() as i32;
        let canvas_height = canvas.height() as i32;
        
        info!("Canvas size: {}x{}", canvas_width, canvas_height);

        let mut total_render_time = Duration::from_ticks(0);
        let mut test_count = 0;

        // Clear canvas to black
        let start_time = Instant::now();
        canvas.clear_color(Rgba8888::new(0, 0, 0, 255));
        let clear_time = start_time.elapsed();
        info!("⏱️  Canvas clear: {:?}", clear_time);
        total_render_time += clear_time;
        test_count += 1;

        // Calculate adaptive coordinates based on canvas size
        let margin = 5;
        let small_size = (canvas_width / 8).max(10);
        let medium_size = (canvas_width / 4).max(20);
        let large_size = (canvas_width / 3).max(30);

        // Test 1: Basic filled rectangles (visible)
        info!("📦 Testing basic filled rectangles...");
        test_count += test_adaptive_filled_rectangles(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 2: CRITICAL - Rounded rectangles (the major bottleneck!)
        info!("🔥 CRITICAL: Testing rounded rectangles (major bottleneck)...");
        test_count += test_adaptive_rounded_rectangles(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 3: Stroked rectangles
        info!("📦 Testing stroked rectangles...");
        test_count += test_adaptive_stroked_rectangles(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 4: Circles (moderate performance)
        info!("⭕ Testing circles...");
        test_count += test_adaptive_circles(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 5: CRITICAL - Lines with different caps (slow thick lines)
        info!("🔥 CRITICAL: Testing lines (thick lines are slow)...");
        test_count += test_adaptive_lines(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 6: CRITICAL - Bezier curves (very slow)
        info!("🔥 CRITICAL: Testing bezier curves (very slow)...");
        test_count += test_adaptive_bezier_curves(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 7: Gradients
        info!("🌈 Testing gradients...");
        test_count += test_adaptive_gradients(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Test 8: Anti-aliasing comparison
        info!("✨ Testing anti-aliasing...");
        test_count += test_adaptive_antialiasing(&mut canvas, &mut total_render_time, canvas_width, canvas_height);

        // Final statistics
        let total_benchmark_time = benchmark_start.elapsed();
        let avg_render_time = if test_count > 0 { total_render_time / test_count as u32 } else { Duration::from_ticks(0) };
        
        info!("📊 ADAPTIVE PERFORMANCE SUMMARY:");
        info!("   Canvas size: {}x{}", canvas_width, canvas_height);
        info!("   Total tests: {}", test_count);
        info!("   Total render time: {:?}", total_render_time);
        info!("   Total benchmark time: {:?}", total_benchmark_time);
        info!("   Average per operation: {:?}", avg_render_time);
        
        // Performance analysis with specific recommendations
        if avg_render_time.as_millis() > 10 {
            warn!("⚠️  CRITICAL: Average render time > 10ms!");
            warn!("   🎯 Focus optimization on: Rounded rectangles, thick lines, bezier curves");
        } else if avg_render_time.as_millis() > 5 {
            warn!("⚠️  Average render time > 5ms - consider optimization");
        } else {
            info!("✅ Performance looks good!");
        }

        // Specific bottleneck analysis
        info!("🔍 BOTTLENECK ANALYSIS:");
        info!("   1. Rounded rectangles: ~93ms each - CRITICAL ISSUE");
        info!("   2. Complex scenes: ~279ms - CRITICAL ISSUE");
        info!("   3. Thick lines: ~22ms each - HIGH PRIORITY");
        info!("   4. Bezier curves: ~17-19ms each - HIGH PRIORITY");
        info!("   5. Basic shapes: <1ms each - GOOD");
    }).await;
    
    // Present the final result
    ctx.request_redraw().await;
    
    info!("🏁 Adaptive Graphics Performance Benchmark Complete");
}

/// Test basic filled rectangles adapted to canvas size
fn test_adaptive_filled_rectangles(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    let margin = 5;
    
    // Small rectangle (red) - top left
    let start = Instant::now();
    Rect::from_coords(margin, margin, (width / 4) as u32, (height / 6) as u32)
        .fill(Paint::solid(Rgba8888::new(255, 0, 0, 255)))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Small filled rect: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Medium rectangle (green) - top right
    let start = Instant::now();
    Rect::from_coords(width / 2, margin, (width / 3) as u32, (height / 4) as u32)
        .fill(Paint::solid(Rgba8888::new(0, 255, 0, 255)))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Medium filled rect: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Large rectangle (blue) - bottom, if space allows
    if height > 60 {
        let start = Instant::now();
        Rect::from_coords(margin, height / 2, (width - 2 * margin) as u32, (height / 3) as u32)
            .fill(Paint::solid(Rgba8888::new(0, 0, 255, 255)))
            .draw(canvas);
        let elapsed = start.elapsed();
        info!("  Large filled rect: {:?}", elapsed);
        *total_time += elapsed;
        test_count += 1;
    }

    test_count
}

/// Test rounded rectangles - THE MAJOR BOTTLENECK
fn test_adaptive_rounded_rectangles(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    let margin = 5;
    
    // Small rounded rectangle - this should be VERY slow based on logs
    let start = Instant::now();
    Rect::from_coords(margin, height / 4, (width / 3) as u32, (height / 6) as u32)
        .fill(Paint::solid(Rgba8888::new(128, 64, 192, 255)))
        .corner_radii(CornerRadii::uniform(FixedI32::<U16>::from_num(3.0))) // Smaller radius for small canvas
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  🔥 Small rounded rect: {:?} (EXPECTED: ~93ms!)", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Only test one rounded rectangle to avoid excessive slowdown
    warn!("  ⚠️  Skipping additional rounded rectangles due to severe performance impact");

    test_count
}

/// Test stroked rectangles adapted to canvas size
fn test_adaptive_stroked_rectangles(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    let margin = 5;
    
    // Thin stroke
    let start = Instant::now();
    Rect::from_coords(margin, height * 2 / 3, (width / 4) as u32, (height / 8) as u32)
        .stroke(Stroke::new(Rgba8888::new(255, 255, 0, 255), 1.0))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Thin stroke rect: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Medium stroke
    let start = Instant::now();
    Rect::from_coords(width / 2, height * 2 / 3, (width / 4) as u32, (height / 8) as u32)
        .stroke(Stroke::new(Rgba8888::new(255, 0, 255, 255), 2.0))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Medium stroke rect: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}

/// Test circles adapted to canvas size
fn test_adaptive_circles(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    let radius = (width / 12).max(8) as f32;
    
    // Small circle
    let start = Instant::now();
    Circle::new(Point::new(width / 6, height / 6), radius)
        .fill(Paint::solid(Rgba8888::new(255, 128, 0, 255)))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Small circle: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Stroked circle
    let start = Instant::now();
    Circle::new(Point::new(width * 5 / 6, height / 6), radius)
        .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 255), 2.0))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Stroked circle: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}

/// Test lines - CRITICAL BOTTLENECK for thick lines
fn test_adaptive_lines(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    
    // Thin line (should be fast)
    let start = Instant::now();
    Line::new(Point::new(10, height / 3), Point::new(width / 3, height / 3))
        .stroke(Stroke::new(Rgba8888::new(255, 0, 0, 255), 1.0))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Thin line: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // Thick line (should be VERY slow based on logs)
    let start = Instant::now();
    Line::new(Point::new(10, height / 2), Point::new(width / 3, height / 2))
        .stroke(Stroke::new(Rgba8888::new(0, 255, 0, 255), 5.0))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  🔥 Thick line: {:?} (EXPECTED: ~22ms!)", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}

/// Test bezier curves - CRITICAL BOTTLENECK
fn test_adaptive_bezier_curves(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    
    // Simple bezier curve (should be VERY slow based on logs)
    let start = Instant::now();
    Bezier::cubic(
        Point::new(10, height * 3 / 4),
        Point::new(width / 4, height * 2 / 3),
        Point::new(width / 3, height * 5 / 6),
        Point::new(width / 2, height * 3 / 4)
    )
    .stroke(Stroke::new(Rgba8888::new(255, 192, 64, 255), 2.0))
    .draw(canvas);
    let elapsed = start.elapsed();
    info!("  🔥 Bezier curve: {:?} (EXPECTED: ~17ms!)", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}

/// Test gradients adapted to canvas size
fn test_adaptive_gradients(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    
    // Linear gradient
    let start = Instant::now();
    Rect::from_coords(width * 2 / 3, height / 4, (width / 4) as u32, (height / 6) as u32)
        .fill(Paint::linear(
            Point::new(width * 2 / 3, height / 4),
            Point::new(width - 5, height / 4),
            Rgba8888::new(255, 0, 0, 255),
            Rgba8888::new(0, 0, 255, 255)
        ))
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Linear gradient: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}

/// Test anti-aliasing performance impact
fn test_adaptive_antialiasing(canvas: &mut Canvas2D, total_time: &mut Duration, width: i32, height: i32) -> u32 {
    let mut test_count = 0;
    let radius = (width / 15).max(6) as f32;
    
    // Without anti-aliasing
    let start = Instant::now();
    Circle::new(Point::new(width / 4, height * 5 / 6), radius)
        .fill(Paint::solid(Rgba8888::new(255, 128, 64, 255)))
        .anti_aliasing(AntiAliasing::None)
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Circle without AA: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    // With anti-aliasing
    let start = Instant::now();
    Circle::new(Point::new(width * 3 / 4, height * 5 / 6), radius)
        .fill(Paint::solid(Rgba8888::new(255, 128, 64, 255)))
        .anti_aliasing(AntiAliasing::High)
        .draw(canvas);
    let elapsed = start.elapsed();
    info!("  Circle with High AA: {:?}", elapsed);
    *total_time += elapsed;
    test_count += 1;

    test_count
}
