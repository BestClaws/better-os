#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use crate::libs::gfx::two_d::{
    Point, Size, Rgba8888, Canvas2D, Rasterizer,
    PrimitiveRect, Circle, Line, Arc, Paint, Stroke, Drawable
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use micromath::F32Ext;

#[embassy_executor::task]
pub async fn demo_fluent_app(context: AppContext) {
    info!("Fluent GFX API demo started");
    let start = Instant::now();
    
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let t = start.elapsed().as_micros() as f32 / 1_000_000.0;
        
        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            
            // Create Canvas2D - the only abstraction layer
            let mut canvas = Canvas2D::new(surface as &mut dyn Rasterizer);
            let w = canvas.width() as i32; 
            let h = canvas.height() as i32;
            
            // Clear with animated background - FLUENT API
            let sweep = ((t * 0.2).sin() * 0.5 + 0.5) as f32;
            let bg_color = Rgba8888::new(
                (20.0 + 120.0 * sweep) as u8,
                (20.0 + 80.0 * (1.0 - sweep)) as u8,
                (40.0 + 100.0 * sweep) as u8,
                255,
            );
            
            PrimitiveRect::new(Point::new(0, 0), Size::new(w as u32, h as u32))
                .fill(Paint::solid(bg_color))
                .draw(&mut canvas);
            
            // Alpha blending demo - overlapping semi-transparent rectangles
            for i in 0..4 {
                let phase = t + i as f32 * 0.8;
                let x = (w / 3) + ((phase * 1.5).cos() * 40.0) as i32;
                let y = (h / 3) + ((phase * 1.2).sin() * 30.0) as i32;
                let size = 60 + (phase.sin() * 20.0) as u32;
                
                // Semi-transparent colors for alpha blending
                let alpha = (128.0 + 100.0 * (phase * 0.7).sin()) as u8;
                let color = Rgba8888::new(
                    (200.0 + 55.0 * (phase).sin()) as u8,
                    (150.0 + 105.0 * (phase + 2.0).sin()) as u8,
                    (100.0 + 155.0 * (phase + 4.0).sin()) as u8,
                    alpha, // Non-opaque for blending!
                );
                
                PrimitiveRect::new(
                    Point::new(x - size as i32 / 2, y - size as i32 / 2), 
                    Size::new(size, size)
                )
                .fill(Paint::solid(color))
                .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 180), 1.5)) // Semi-transparent stroke
                .corner_radius(12.0) // Larger radius to show AA
                .draw(&mut canvas);
            }
            
            // Asymmetric corner radii with gradient example
            let gradient_rect_x = w * 2 / 3;
            let gradient_rect_y = h / 4;
             PrimitiveRect::new(
                Point::new(gradient_rect_x, gradient_rect_y), 
                Size::new(120, 80)
            )
            .fill(Paint::linear(
                Point::new(gradient_rect_x, gradient_rect_y),
                Point::new(gradient_rect_x + 120, gradient_rect_y + 80),
                Rgba8888::new(255, 100, 150, 200), // Semi-transparent start
                Rgba8888::new(100, 150, 255, 220)  // Semi-transparent end
            ))
            .corner_radius(25.0) // Large radius to showcase smooth AA
            .draw(&mut canvas);
            
            // Circles with smooth radial gradients and alpha blending
            for i in 0..3 {
                let phase = t * 0.8 + i as f32 * 2.0;
                let x = (w / 6) + ((phase).cos() * 35.0) as i32;
                let y = (h * 3 / 4) + ((phase).sin() * 25.0) as i32;
                let radius = 20.0 + (phase * 2.0).sin() * 8.0;
                
                Circle::new(Point::new(x, y), radius)
                    .fill(Paint::radial(
                        Point::new(x, y), 
                        radius,
                        Rgba8888::new(255, 200, 100, 180), // Semi-transparent center
                        Rgba8888::new(100, 100, 255, 80)   // More transparent edge
                    ))
                    .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 120), 2.0)) // AA stroke
                    .draw(&mut canvas);
            }
            
            // Anti-aliased lines with varying transparency
            for i in 0..8 {
                let angle = (i as f32 / 8.0) * 2.0 * core::f32::consts::PI + t * 0.5;
                let center_x = w * 4 / 5;
                let center_y = h / 2;
                let start_radius = 15.0;
                let end_radius = 45.0;
                
                let start = Point::new(
                    center_x + (angle.cos() * start_radius) as i32,
                    center_y + (angle.sin() * start_radius) as i32,
                );
                let end = Point::new(
                    center_x + (angle.cos() * end_radius) as i32,
                    center_y + (angle.sin() * end_radius) as i32,
                );
                
                let alpha = (150.0 + 105.0 * (t + i as f32 * 0.5).sin()) as u8;
                let color = Rgba8888::new(
                    (128.0 + 127.0 * (t + i as f32 * 0.3).sin()) as u8,
                    (200.0 + 55.0 * (t + i as f32 * 0.8).cos()) as u8,
                    (128.0 + 127.0 * (t + i as f32 * 0.7).sin()) as u8,
                    alpha, // Varying transparency
                );
                
                Line::new(start, end)
                    .stroke(Stroke::new(color, 2.5)) // AA enabled by default
                    .draw(&mut canvas);
            }
            
            // Arcs - new primitive
            for i in 0..4 {
                let phase = t * 0.6 + i as f32 * 1.5;
                let center_x = w / 2;
                let center_y = h / 2;
                let radius = 40.0 + i as f32 * 10.0;
                let start_angle = phase;
                let end_angle = phase + core::f32::consts::PI * 0.5;
                
                let color = Rgba8888::new(
                    255,
                    (128.0 + 127.0 * (phase * 0.8).sin()) as u8,
                    (128.0 + 127.0 * (phase * 1.2).cos()) as u8,
                    180,
                );
                
                Arc::new(Point::new(center_x, center_y), radius, start_angle, end_angle)
                    .stroke(Stroke::new(color, 4.0))
                    .draw(&mut canvas);
            }
            
            // Smooth gradient showcase with high-quality interpolation
             PrimitiveRect::new(Point::new(10, h - 60), Size::new(w as u32 - 20, 50))
                .fill(Paint::linear(
                    Point::new(10, h - 60),
                    Point::new(w - 10, h - 60),
                    Rgba8888::new(255, 100, 100, 220), // Semi-transparent
                    Rgba8888::new(100, 255, 150, 180)  // Different alpha for smooth blend
                ))
                .corner_radius(15.0) // Large radius for AA demonstration
                .draw(&mut canvas);
                
            // Multiple overlapping gradient rectangles to show alpha compositing
            let overlap_y = 10;
            for i in 0..3 {
                let offset_x = 10 + i * 25;
                PrimitiveRect::new(Point::new(offset_x, overlap_y), Size::new(60, 40))
                    .fill(Paint::linear(
                        Point::new(offset_x, overlap_y),
                        Point::new(offset_x + 60, overlap_y + 40),
                        Rgba8888::new((255 - i * 50) as u8, (100 + i * 50) as u8, (150 + i * 30) as u8, 160),
                        Rgba8888::new((100 + i * 40) as u8, (200 - i * 30) as u8, (255 - i * 60) as u8, 140)
                    ))
                    .corner_radius(8.0 + i as f32 * 3.0) // Different radii for each
                    .draw(&mut canvas);
            }
            
            let draw_time = draw_start.elapsed().as_micros();
            if draw_time > 5000 {
                info!("Fluent draw time: {} us", draw_time);
            }
        }).await;
        
        Timer::after(Duration::from_millis(16)).await;
    }
}
