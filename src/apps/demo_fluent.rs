#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use crate::libs::gfx::two_d::{
    Point, Size, Rgba8888, Canvas2D, Rasterizer,
    PrimitiveRect, Circle, Line, Arc, Bezier, Paint, Stroke, Drawable
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use micromath::F32Ext;

#[embassy_executor::task]
pub async fn demo_fluent_app(context: AppContext) {
    info!("Comprehensive Fluent GFX API Demo Started - Showcasing All Features!");
    let start = Instant::now();
    
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let t = start.elapsed().as_micros() as f32 / 1_000_000.0;
        
        // Cycle through different demos every 2 seconds
        let demo_phase = ((t / 2.0) as u32) % 8;
        let demo_t = (t % 2.0) / 2.0; // 0.0 to 1.0 within each demo
        
        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            
            // Create Canvas2D - the only abstraction layer
            let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
            let w = canvas.width() as i32; 
            let h = canvas.height() as i32;
            
            // Clear with animated background
            let bg_sweep = (demo_t * core::f32::consts::PI).sin() * 0.5 + 0.5;
            let bg_color = Rgba8888::new(
                (15.0 + 25.0 * bg_sweep) as u8,
                (15.0 + 25.0 * bg_sweep) as u8,
                (25.0 + 35.0 * bg_sweep) as u8,
                255,
            );
            canvas.clear_color(bg_color);
            
            match demo_phase {
                0 => demo_basic_rectangles(&mut canvas, w, h, demo_t),
                1 => demo_rounded_rectangles(&mut canvas, w, h, demo_t),
                2 => demo_alpha_blending(&mut canvas, w, h, demo_t),
                3 => demo_gradient_fills(&mut canvas, w, h, demo_t),
                4 => demo_circles(&mut canvas, w, h, demo_t),
                5 => demo_lines(&mut canvas, w, h, demo_t),
                6 => demo_arcs(&mut canvas, w, h, demo_t),
                7 => demo_bezier_curves(&mut canvas, w, h, demo_t),
                _ => {}
            }
            
            // Show demo title
            draw_demo_title(&mut canvas, w, h, demo_phase);
            
            let draw_time = draw_start.elapsed().as_micros();
            if draw_time > 8000 {
                info!("Demo {} draw time: {} us", demo_phase, draw_time);
            }
        }).await;
        
        Timer::after(Duration::from_millis(16)).await;
    }
}

fn demo_basic_rectangles(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Basic solid rectangles with different sizes and colors
    for i in 0..6 {
        let phase = t * 2.0 + i as f32 * 0.5;
        let x = 50 + (i % 3) * (w / 4);
        let y = 50 + (i / 3) * (h / 3);
        let size = 60 + (phase.sin() * 20.0) as u32;
        
        let color = Rgba8888::new(
            (100 + i * 25) as u8,
            (150.0 + (phase.sin() * 50.0)) as u8,
            (200 - i * 20) as u8,
            255,
        );
        
        PrimitiveRect::new(
            Point::new(x - size as i32 / 2, y - size as i32 / 2),
            Size::new(size, size)
        )
        .fill(Paint::solid(color))
        .stroke(Stroke::new(Rgba8888::white(), 2.0))
        .draw(canvas);
    }
}

fn demo_rounded_rectangles(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Rounded rectangles with different corner radii
    for i in 0..4 {
        let phase = t * 1.5 + i as f32 * 0.8;
        let x = 60 + (i % 2) * ((w / 2).saturating_sub(60));
        let y = 60 + (i / 2) * ((h / 2).saturating_sub(60));
        let width = 120 + (phase.cos() * 30.0) as u32;
        let height = 80 + (phase.sin() * 20.0) as u32;
        
        let radius = 5.0 + i as f32 * 8.0 + (phase * 2.0).sin() * 10.0;
        
        let color = Rgba8888::new(
            (180.0 + (phase * 3.0).sin() * 75.0) as u8,
            (120.0 + (phase * 2.0).cos() * 80.0) as u8,
            (160.0 + (phase * 1.5).sin() * 95.0) as u8,
            255,
        );
        
        PrimitiveRect::new(Point::new(x, y), Size::new(width, height))
            .fill(Paint::solid(color))
            .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 200), 1.5))
            .corner_radius(radius)
            .draw(canvas);
    }
    
    // Showcase different corner radii on same rectangle
    let center_x = (w / 2).saturating_sub(60);
    let center_y = (h / 2).saturating_sub(40);
    PrimitiveRect::new(Point::new(center_x, center_y), Size::new(120, 80))
        .fill(Paint::solid(Rgba8888::new(255, 180, 100, 255)))
        .stroke(Stroke::new(Rgba8888::new(100, 50, 200, 255), 3.0))
        .corner_radius(20.0 + (t * 4.0).sin() * 15.0) // Animated radius
        .draw(canvas);
}

fn demo_alpha_blending(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Overlapping semi-transparent rectangles to show alpha blending
    for i in 0..8 {
        let phase = t * 3.0 + i as f32 * 0.4;
        let x = (w / 2) + ((phase * 1.2).cos() * 80.0) as i32;
        let y = (h / 2) + ((phase * 0.8).sin() * 60.0) as i32;
        let size = 50 + (phase.sin() * 25.0) as u32;
        
        // Varying transparency levels
        let alpha = (80.0 + 120.0 * ((phase * 0.7).sin() * 0.5 + 0.5)) as u8;
        let color = Rgba8888::new(
            (200.0 + 55.0 * (phase + i as f32).sin()) as u8,
            (150.0 + 105.0 * (phase + i as f32 + 2.0).sin()) as u8,
            (100.0 + 155.0 * (phase + i as f32 + 4.0).sin()) as u8,
            alpha,
        );
        
        PrimitiveRect::new(
            Point::new(x - size as i32 / 2, y - size as i32 / 2),
            Size::new(size, size)
        )
        .fill(Paint::solid(color))
        .corner_radius(15.0)
        .draw(canvas);
    }
}

fn demo_gradient_fills(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Linear gradients
    for i in 0..3 {
        let x = 40 + i * (w / 4);
        let y = 40;
        let width = 80;
        let height = 60;
        
        let phase = t * 2.0 + i as f32 * 2.0;
        let start_color = Rgba8888::new(
            (255.0 * (phase.sin() * 0.5 + 0.5)) as u8,
            (255.0 * (phase.cos() * 0.5 + 0.5)) as u8,
            200,
            255,
        );
        let end_color = Rgba8888::new(
            100,
            (255.0 * ((phase + 1.0).sin() * 0.5 + 0.5)) as u8,
            (255.0 * ((phase + 2.0).cos() * 0.5 + 0.5)) as u8,
            255,
        );
        
        PrimitiveRect::new(Point::new(x, y), Size::new(width, height))
            .fill(Paint::linear(
                Point::new(x, y),
                Point::new(x + width as i32, y + height as i32),
                start_color,
                end_color,
            ))
            .corner_radius(10.0)
            .draw(canvas);
    }
    
    // Radial gradients
    for i in 0..3 {
        let x = 60 + i * (w / 4);
        let y = h / 2;
        let radius = 35.0 + (t * 3.0 + i as f32).sin() * 10.0;
        
        let center_color = Rgba8888::new(255, 255, 100, 200);
        let edge_color = Rgba8888::new(
            100,
            (150.0 + 105.0 * (t + i as f32).cos()) as u8,
            255,
            50,
        );
        
        PrimitiveRect::new(
            Point::new(x - 40, y - 40),
            Size::new(80, 80)
        )
        .fill(Paint::radial(
            Point::new(x, y),
            radius,
            center_color,
            edge_color,
        ))
        .corner_radius(20.0)
        .draw(canvas);
    }
    
    // Large gradient showcase at bottom
    PrimitiveRect::new(Point::new(20, h.saturating_sub(80)), Size::new((w.saturating_sub(40)) as u32, 60))
        .fill(Paint::linear(
            Point::new(20, h.saturating_sub(80)),
            Point::new(w.saturating_sub(20), h.saturating_sub(20)),
            Rgba8888::new(255, 100, 150, 180),
            Rgba8888::new(100, 255, 200, 220),
        ))
        .corner_radius(25.0)
        .draw(canvas);
}

fn demo_circles(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Solid circles
    for i in 0..5 {
        let phase = t * 2.5 + i as f32 * 1.2;
        let x = 80 + (i % 3) * (w / 4);
        let y = 80 + (i / 3) * (h / 3);
        let radius = 25.0 + (phase.sin() * 15.0);
        
        let color = Rgba8888::new(
            (150.0 + (phase * 2.0).sin() * 105.0) as u8,
            (150.0 + (phase * 1.5).cos() * 105.0) as u8,
            (150.0 + (phase * 3.0).sin() * 105.0) as u8,
            255,
        );
        
        Circle::new(Point::new(x, y), radius)
            .fill(Paint::solid(color))
            .stroke(Stroke::new(Rgba8888::white(), 2.0))
            .draw(canvas);
    }
    
    // Gradient-filled circles
    for i in 0..3 {
        let phase = t * 1.8 + i as f32 * 2.0;
        let x = (w / 2) + ((phase).cos() * 60.0) as i32;
        let y = (h / 2) + ((phase).sin() * 40.0) as i32;
        let radius = 30.0 + (phase * 2.0).sin() * 10.0;
        
        Circle::new(Point::new(x, y), radius)
            .fill(Paint::radial(
                Point::new(x, y),
                radius,
                Rgba8888::new(255, 200, 100, 200),
                Rgba8888::new(100, 150, 255, 100),
            ))
            .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 150), 1.5))
            .draw(canvas);
    }
}

fn demo_lines(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Radiating lines from center
    let center_x = w / 2;
    let center_y = h / 2;
    
    for i in 0..12 {
        let angle = (i as f32 / 12.0) * 2.0 * core::f32::consts::PI + t * 2.0;
        let start_radius = 20.0;
        let end_radius = 80.0 + (t * 3.0 + i as f32 * 0.5).sin() * 30.0;
        
        let start = Point::new(
            center_x + (angle.cos() * start_radius) as i32,
            center_y + (angle.sin() * start_radius) as i32,
        );
        let end = Point::new(
            center_x + (angle.cos() * end_radius) as i32,
            center_y + (angle.sin() * end_radius) as i32,
        );
        
        let color = Rgba8888::new(
            (128.0 + 127.0 * (t + i as f32 * 0.3).sin()) as u8,
            (128.0 + 127.0 * (t + i as f32 * 0.5).cos()) as u8,
            (128.0 + 127.0 * (t + i as f32 * 0.7).sin()) as u8,
            (150.0 + 105.0 * (t + i as f32 * 0.2).sin()) as u8,
        );
        
        Line::new(start, end)
            .stroke(Stroke::new(color, 2.0 + (i as f32 * 0.3)))
            .draw(canvas);
    }
    
    // Grid of lines
    for i in 0..8 {
        let x = 50 + i * (w.saturating_sub(100)) / 7;
        let y_start = h / 4;
        let y_end = h * 3 / 4;
        
        let phase = t * 4.0 + i as f32 * 0.5;
        let color = Rgba8888::new(
            255,
            (128.0 + 127.0 * phase.sin()) as u8,
            (128.0 + 127.0 * phase.cos()) as u8,
            180,
        );
        
        Line::new(Point::new(x, y_start), Point::new(x, y_end))
            .stroke(Stroke::new(color, 1.5))
            .draw(canvas);
    }
}

fn demo_arcs(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Concentric arcs
    for i in 0..6 {
        let phase = t * 2.0 + i as f32 * 0.8;
        let center_x = w / 2;
        let center_y = h / 2;
        let radius = 30.0 + i as f32 * 15.0;
        let start_angle = phase;
        let end_angle = phase + core::f32::consts::PI * 0.75;
        
        let color = Rgba8888::new(
            (255.0 * ((phase * 0.8).sin() * 0.5 + 0.5)) as u8,
            (255.0 * ((phase * 1.2).cos() * 0.5 + 0.5)) as u8,
            (255.0 * ((phase * 1.5).sin() * 0.5 + 0.5)) as u8,
            200,
        );
        
        Arc::new(Point::new(center_x, center_y), radius, start_angle, end_angle)
            .stroke(Stroke::new(color, 3.0 + i as f32 * 0.5))
            .draw(canvas);
    }
    
    // Animated arc segments in corners
    let corners = [
        (w / 4, h / 4),
        (w * 3 / 4, h / 4),
        (w / 4, h * 3 / 4),
        (w * 3 / 4, h * 3 / 4),
    ];
    
    for (i, &(cx, cy)) in corners.iter().enumerate() {
        let phase = t * 3.0 + i as f32 * 1.5;
        let radius = 40.0;
        let start_angle = phase;
        let end_angle = phase + core::f32::consts::PI * 0.5;
        
        let color = Rgba8888::new(
            255,
            (100 + i * 40) as u8,
            (200 - i * 30) as u8,
            255,
        );
        
        Arc::new(Point::new(cx, cy), radius, start_angle, end_angle)
            .stroke(Stroke::new(color, 4.0))
            .draw(canvas);
    }
}

fn demo_bezier_curves(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    // Quadratic bezier curves
    for i in 0..4 {
        let phase = t * 2.0 + i as f32 * 1.5;
        
        let start = Point::new(50 + i * (w / 5), h / 3);
        let control = Point::new(
            50 + i * (w / 5) + (phase.sin() * 50.0) as i32,
            (h / 3).saturating_sub(60) + (phase.cos() * 40.0) as i32,
        );
        let end = Point::new(50 + i * (w / 5) + 80, h / 3);
        
        let color = Rgba8888::new(
            (255.0 * ((phase * 0.7).sin() * 0.5 + 0.5)) as u8,
            (255.0 * ((phase * 1.1).cos() * 0.5 + 0.5)) as u8,
            (255.0 * ((phase * 1.3).sin() * 0.5 + 0.5)) as u8,
            255,
        );
        
        Bezier::quadratic(start, control, end)
            .stroke(Stroke::new(color, 3.0))
            .draw(canvas);
    }
    
    // Cubic bezier curves
    for i in 0..3 {
        let phase = t * 1.5 + i as f32 * 2.0;
        
        let start = Point::new(60 + i * (w / 4), h * 2 / 3);
        let control1 = Point::new(
            60 + i * (w / 4) + (phase.sin() * 60.0) as i32,
            (h * 2 / 3).saturating_sub(80) + (phase.cos() * 30.0) as i32,
        );
        let control2 = Point::new(
            60 + i * (w / 4) + 60 + ((phase + 1.0).sin() * 40.0) as i32,
            (h * 2 / 3).saturating_sub(40) + ((phase + 1.0).cos() * 50.0) as i32,
        );
        let end = Point::new(60 + i * (w / 4) + 120, h * 2 / 3);
        
        let color = Rgba8888::new(
            255,
            (128.0 + 127.0 * (phase * 0.8).sin()) as u8,
            (128.0 + 127.0 * (phase * 1.2).cos()) as u8,
            200,
        );
        
        Bezier::cubic(start, control1, control2, end)
            .stroke(Stroke::new(color, 2.5))
            .draw(canvas);
    }
    
    // Animated flowing curve
    let wave_points = (0..8).map(|i| {
        let x = 40 + i * (w.saturating_sub(80)) / 7;
        let y = h.saturating_sub(60) + ((t * 4.0 + i as f32 * 0.5).sin() * 30.0) as i32;
        Point::new(x, y)
    }).collect::<heapless::Vec<Point, 8>>();
    
    for i in 0..(wave_points.len() - 1) {
        if i + 1 < wave_points.len() {
            let start = wave_points[i];
            let end = wave_points[i + 1];
            let control = Point::new(
                (start.x + end.x) / 2,
                ((start.y + end.y) / 2).saturating_sub(20),
            );
            
            Bezier::quadratic(start, control, end)
                .stroke(Stroke::new(Rgba8888::new(100, 255, 200, 255), 3.0))
                .draw(canvas);
        }
    }
}

fn draw_demo_title(canvas: &mut Canvas2D, w: i32, h: i32, demo_phase: u32) {
    let title = match demo_phase {
        0 => "BASIC RECTANGLES",
        1 => "ROUNDED RECTANGLES",
        2 => "ALPHA BLENDING",
        3 => "GRADIENT FILLS",
        4 => "CIRCLES",
        5 => "LINES",
        6 => "ARCS",
        7 => "BEZIER CURVES",
        _ => "UNKNOWN",
    };
    
    // Draw title background
    let title_width = (title.len() as u32 * 8 + 20).min(w as u32 - 10);
    let title_x = if w as u32 > title_width {
        (w as u32 - title_width) / 2
    } else {
        5
    };
    
    PrimitiveRect::new(
        Point::new(title_x as i32, 5),
        Size::new(title_width, 25)
    )
    .fill(Paint::solid(Rgba8888::new(0, 0, 0, 180)))
    .corner_radius(5.0)
    .draw(canvas);
    
    // Note: Text rendering would need a font system - for now just the background
    info!("Demo: {}", title);
}
