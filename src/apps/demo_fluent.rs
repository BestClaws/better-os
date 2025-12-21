#![no_std]

use embassy_time::{Duration, Instant, Timer};
use defmt::info;
use crate::libs::gfx::two_d::{
    AntiAliasing, Arc, Bezier, Canvas2D,
    Circle, CornerRadii, Drawable, Line, Paint, PrimitiveRect, Rasterizer, Rgba8888, Stroke
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use micromath::F32Ext;
use crate::util::math::primitives::{Point, Size};

// Easing functions for smooth animations
fn ease_in_out_sine(t: f32) -> f32 {
    -(t * core::f32::consts::PI).cos() / 2.0 + 0.5
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn smooth_lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * ease_in_out_sine(t)
}

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
        
        // Cycle through different demos every 4 seconds for smoother transitions
        let demo_phase = ((t / 4.0) as u32) % 7;
        let raw_demo_t = (t % 4.0) / 4.0; // 0.0 to 1.0 within each demo
        let demo_t = ease_in_out_cubic(raw_demo_t); // Apply easing for smoother transitions
        
        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            
            // Create Canvas2D - the only abstraction layer
            let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
            let w = canvas.width() as i32; 
            let h = canvas.height() as i32;
            
            // Clear with dark background
            canvas.clear_color(Rgba8888::new(20, 25, 35, 255));
            
            match demo_phase {
                0 => demo_antialiased_shapes(&mut canvas, w, h, demo_t),
                1 => demo_alpha_blending_rects(&mut canvas, w, h, demo_t),
                2 => demo_arcs(&mut canvas, w, h, demo_t),
                3 => demo_bezier_curves(&mut canvas, w, h, demo_t),
                4 => demo_radial_gradients(&mut canvas, w, h, demo_t),
                5 => demo_linear_gradients(&mut canvas, w, h, demo_t),
                6 => demo_performance_clock(&mut canvas, w, h, t),
                _ => {}
            }
            
            // No title bar needed
            
            let draw_time = draw_start.elapsed().as_micros();
            info!("Demo {} render time: {} μs", demo_phase, draw_time);
        }).await;
        
            Timer::after(Duration::from_millis(8)).await; // Increased refresh rate for smoother animations
    }
}// Demo 1: Anti-aliased shapes showcase
fn demo_antialiased_shapes(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Anti-aliased circle
    Circle::new(Point::new(center_x - 80, center_y), 40.0)
        .fill(Paint::solid(Rgba8888::new(100, 150, 255, 255)))
        .stroke(Stroke::new(Rgba8888::white(), 2.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Anti-aliased line
    Line::new(
        Point::new(center_x - 40, center_y - 60),
        Point::new(center_x + 40, center_y + 60)
    )
    .stroke(Stroke::new(Rgba8888::new(255, 100, 100, 255), 4.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
    
    // Anti-aliased arc
    let arc_phase = t * 2.0;
    Arc::new(Point::new(center_x + 80, center_y), 35.0, arc_phase, arc_phase + core::f32::consts::PI)
        .stroke(Stroke::new(Rgba8888::new(100, 255, 150, 255), 3.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
}

// Demo 2: Alpha blending rectangles
fn demo_alpha_blending_rects(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Overlapping rectangles with different alpha values
    for i in 0..4 {
        let base_angle = i as f32 * core::f32::consts::PI / 2.0;
        let angle = base_angle + t * core::f32::consts::PI * 2.0;
        let smooth_angle = smooth_lerp(base_angle, angle, t);
        let offset_x = (angle.cos() * 60.0) as i32;
        let offset_y = (angle.sin() * 60.0) as i32;
        
        let alpha = (100 + i * 40) as u8;
        let color = match i {
            0 => Rgba8888::new(255, 100, 100, alpha), // Red
            1 => Rgba8888::new(100, 255, 100, alpha), // Green  
            2 => Rgba8888::new(100, 100, 255, alpha), // Blue
            _ => Rgba8888::new(255, 255, 100, alpha), // Yellow
        };
        
        PrimitiveRect::new(
            Point::new(center_x + offset_x - 40, center_y + offset_y - 40),
            Size::new(80, 80)
        )
        .fill(Paint::solid(color))
        .corner_radius(10.0)
        .aa(AntiAliasing::Low)
        .draw(canvas);
    }
}

// Demo 4: Arcs showcase
fn demo_arcs(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Smoothly animated rotating arc with continuous motion
    let base_rotation = t * core::f32::consts::PI; // Slower base rotation
    let smooth_rotation = ease_in_out_sine(t) * core::f32::consts::PI * 2.0; // Add smooth oscillation
    let final_rotation = base_rotation + smooth_rotation;
    
    // Main arc with smooth sweep
    let sweep_amount = smooth_lerp(core::f32::consts::PI, core::f32::consts::PI * 2.0, ease_in_out_sine(t));
    Arc::new(
        Point::new(center_x, center_y), 
        60.0, 
        final_rotation, 
        final_rotation + sweep_amount
    )
    .stroke(Stroke::new(Rgba8888::new(255, 150, 100, 255), 6.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
    
    // Concentric arcs with different colors and smooth independent rotations
    for i in 0..3 {
        let radius = 80.0 + i as f32 * 20.0;
        let phase_offset = i as f32 * core::f32::consts::PI / 3.0;
        let individual_rotation = phase_offset + smooth_lerp(0.0, final_rotation * 0.5, ease_in_out_sine(t));
        let sweep = smooth_lerp(core::f32::consts::PI * 0.5, core::f32::consts::PI * 0.8, ease_in_out_sine(t));
        
        // Smooth color transitions based on rotation
        let alpha = (200.0 + ease_in_out_sine(t) * 55.0) as u8;
        let color = match i {
            0 => Rgba8888::new(255, 100, 100, alpha),
            1 => Rgba8888::new(100, 255, 100, alpha),
            _ => Rgba8888::new(100, 100, 255, alpha),
        };
        
        Arc::new(
            Point::new(center_x, center_y), 
            radius, 
            individual_rotation, 
            individual_rotation + sweep
        )
        .stroke(Stroke::new(color, 3.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    }
    
    // Add subtle pulsing background arc for depth
    let bg_radius = 100.0 + ease_in_out_sine(t) * 10.0;
    Arc::new(
        Point::new(center_x, center_y),
        bg_radius,
        0.0,
        core::f32::consts::PI * 2.0
    )
    .stroke(Stroke::new(Rgba8888::new(100, 100, 100, 50), 1.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
}

// Demo 5: Bezier curves showcase
fn demo_bezier_curves(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Animated quadratic bezier with smooth easing
    let control_offset = (t * core::f32::consts::PI * 2.0).sin() * 80.0 * ease_in_out_sine(t);
    Bezier::quadratic(
        Point::new(center_x - 100, center_y),
        Point::new(center_x, center_y - control_offset as i32),
        Point::new(center_x + 100, center_y)
    )
    .stroke(Stroke::new(Rgba8888::new(255, 200, 100, 255), 4.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
    
    // Animated cubic bezier
    let phase = t * 2.0;
    Bezier::cubic(
        Point::new(center_x - 80, center_y + 60),
        Point::new(center_x - 40 + (phase.sin() * 40.0) as i32, center_y - 40),
        Point::new(center_x + 40 + (phase.cos() * 40.0) as i32, center_y - 40),
        Point::new(center_x + 80, center_y + 60)
    )
    .stroke(Stroke::new(Rgba8888::new(100, 200, 255, 255), 3.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
}

// Demo 6: Radial gradients showcase
fn demo_radial_gradients(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Large radial gradient circle with smooth pulsing
    let base_radius = 80.0;
    let pulse = (t * core::f32::consts::PI).sin() * 20.0;
    let radius = base_radius + smooth_lerp(0.0, pulse, ease_in_out_sine(t));
    Circle::new(Point::new(center_x, center_y), radius)
        .fill(Paint::radial(
            Point::new(center_x, center_y),
            radius,
            Rgba8888::new(255, 255, 100, 255), // Bright center
            Rgba8888::new(255, 100, 100, 100), // Transparent edge
        ))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Smaller radial gradient rectangles
    for i in 0..3 {
        let angle = i as f32 * 2.0 * core::f32::consts::PI / 3.0 + t;
        let x = center_x + (angle.cos() * 120.0) as i32;
        let y = center_y + (angle.sin() * 120.0) as i32;
        
        PrimitiveRect::new(Point::new(x - 30, y - 30), Size::new(60, 60))
            .fill(Paint::radial(
                Point::new(x, y),
                40.0,
                Rgba8888::new(100, 255, 255, 200),
                Rgba8888::new(100, 100, 255, 50),
            ))
            .corner_radius(15.0)
            .aa(AntiAliasing::Low)
            .draw(canvas);
    }
}

// Demo 7: Linear gradients showcase  
fn demo_linear_gradients(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Rotating linear gradient rectangle
    let angle = t * 1.5;
    let gradient_end_x = center_x + (angle.cos() * 100.0) as i32;
    let gradient_end_y = center_y + (angle.sin() * 100.0) as i32;
    
    PrimitiveRect::new(
        Point::new(center_x - 80, center_y - 60),
        Size::new(160, 120)
    )
    .fill(Paint::linear(
        Point::new(center_x - 80, center_y - 60),
        Point::new(gradient_end_x, gradient_end_y),
        Rgba8888::new(255, 100, 150, 255),
        Rgba8888::new(100, 255, 150, 255),
    ))
    .corner_radius(20.0)
    .stroke(Stroke::new(Rgba8888::white(), 2.0))
    .aa(AntiAliasing::Low)
    .draw(canvas);
}

// Demo 8: Performance test - Animated clock
fn demo_performance_clock(canvas: &mut Canvas2D, w: i32, h: i32, t: f32) {
    let center_x = w / 2;
    let center_y = h / 2;
    let clock_radius = 100.0;
    
    // Clock face with radial gradient
    Circle::new(Point::new(center_x, center_y), clock_radius)
        .fill(Paint::radial(
            Point::new(center_x, center_y),
            clock_radius,
            Rgba8888::new(250, 250, 250, 255),
            Rgba8888::new(200, 200, 200, 255),
        ))
        .stroke(Stroke::new(Rgba8888::new(100, 100, 100, 255), 3.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Hour markers
    for i in 0..12 {
        let angle = i as f32 * core::f32::consts::PI / 6.0 - core::f32::consts::PI / 2.0;
        let inner_radius = clock_radius - 15.0;
        let outer_radius = clock_radius - 5.0;
        
        let x1 = center_x + (angle.cos() * inner_radius) as i32;
        let y1 = center_y + (angle.sin() * inner_radius) as i32;
        let x2 = center_x + (angle.cos() * outer_radius) as i32;
        let y2 = center_y + (angle.sin() * outer_radius) as i32;
        
        Line::new(Point::new(x1, y1), Point::new(x2, y2))
            .stroke(Stroke::new(Rgba8888::new(100, 100, 100, 255), 2.0))
            .aa(AntiAliasing::Low)
            .draw(canvas);
    }
    
    // Clock hands (animated)
    let seconds = (t * 6.0) % 60.0; // 6x speed for demo
    let minutes = (t * 0.1) % 60.0;
    let hours = (t * 0.008333) % 12.0;
    
    // Hour hand
    let hour_angle = hours * core::f32::consts::PI / 6.0 - core::f32::consts::PI / 2.0;
    let hour_x = center_x + (hour_angle.cos() * 50.0) as i32;
    let hour_y = center_y + (hour_angle.sin() * 50.0) as i32;
    Line::new(Point::new(center_x, center_y), Point::new(hour_x, hour_y))
        .stroke(Stroke::new(Rgba8888::new(100, 100, 100, 255), 4.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Minute hand
    let minute_angle = minutes * core::f32::consts::PI / 30.0 - core::f32::consts::PI / 2.0;
    let minute_x = center_x + (minute_angle.cos() * 70.0) as i32;
    let minute_y = center_y + (minute_angle.sin() * 70.0) as i32;
    Line::new(Point::new(center_x, center_y), Point::new(minute_x, minute_y))
        .stroke(Stroke::new(Rgba8888::new(50, 50, 50, 255), 3.0))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Second hand
    let second_angle = seconds * core::f32::consts::PI / 30.0 - core::f32::consts::PI / 2.0;
    let second_x = center_x + (second_angle.cos() * 85.0) as i32;
    let second_y = center_y + (second_angle.sin() * 85.0) as i32;
    Line::new(Point::new(center_x, center_y), Point::new(second_x, second_y))
        .stroke(Stroke::new(Rgba8888::new(255, 50, 50, 255), 1.5))
        .aa(AntiAliasing::Low)
        .draw(canvas);
    
    // Center dot
    Circle::new(Point::new(center_x, center_y), 5.0)
        .fill(Paint::solid(Rgba8888::new(100, 100, 100, 255)))
        .aa(AntiAliasing::Low)
        .draw(canvas);
}

