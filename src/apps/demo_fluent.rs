#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use crate::libs::gfx::two_d::{
    Point, Size, Rgba8888, Canvas2D, Rasterizer,
    FluentRect, Circle, Line, Arc, Paint, Stroke
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
            
            FluentRect::new(Point::new(0, 0), Size::new(w as u32, h as u32))
                .fill(Paint::solid(bg_color))
                .draw(&mut canvas);
            
            // Animated rectangles - YOUR DESIRED API
            for i in 0..5 {
                let phase = t + i as f32 * 0.5;
                let x = (w / 2) + ((phase * 2.0).cos() * 30.0) as i32;
                let y = (h / 2) + ((phase * 1.5).sin() * 20.0) as i32;
                let size = 20 + (phase.sin() * 10.0) as u32;
                
                let color = Rgba8888::new(
                    (128.0 + 127.0 * (phase).sin()) as u8,
                    (128.0 + 127.0 * (phase + 2.0).sin()) as u8,
                    (128.0 + 127.0 * (phase + 4.0).sin()) as u8,
                    255,
                );
                
                // EXACTLY what you wanted!
                FluentRect::new(
                    Point::new(x - size as i32 / 2, y - size as i32 / 2), 
                    Size::new(size, size)
                )
                .fill(Paint::solid(color))
                .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 2.0))
                .corner_radius(5.0)
                .draw(&mut canvas);
            }
            
            // Circles with gradient fills
            for i in 0..3 {
                let phase = t * 0.8 + i as f32 * 2.0;
                let x = (w / 4) + ((phase).cos() * 40.0) as i32;
                let y = (h / 4) + ((phase).sin() * 40.0) as i32;
                let radius = 15.0 + (phase * 2.0).sin() * 5.0;
                
                Circle::new(Point::new(x, y), radius)
                    .fill(Paint::radial(
                        Point::new(x, y), 
                        radius,
                        Rgba8888::opaque(255, 200, 100),
                        Rgba8888::opaque(100, 100, 255)
                    ))
                    .draw(&mut canvas);
            }
            
            // Lines - clean and simple
            for i in 0..8 {
                let angle = (i as f32 / 8.0) * 2.0 * core::f32::consts::PI + t * 0.5;
                let center_x = w * 3 / 4;
                let center_y = h * 3 / 4;
                let start_radius = 10.0;
                let end_radius = 30.0;
                
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
                    255,
                    (128.0 + 127.0 * (t + i as f32 * 0.7).sin()) as u8,
                    255,
                );
                
                Line::new(start, end)
                    .stroke(Stroke::new(color, 3.0))
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
            
            // Linear gradient rectangle
            FluentRect::new(Point::new(10, 10), Size::new(80, 30))
                .fill(Paint::linear(
                    Point::new(10, 10),
                    Point::new(90, 10),
                    Rgba8888::opaque(255, 100, 100),
                    Rgba8888::opaque(100, 100, 255)
                ))
                .corner_radius(8.0)
                .draw(&mut canvas);
            
            let draw_time = draw_start.elapsed().as_micros();
            if draw_time > 5000 {
                info!("Fluent draw time: {} us", draw_time);
            }
        }).await;
        
        Timer::after(Duration::from_millis(16)).await;
    }
}
