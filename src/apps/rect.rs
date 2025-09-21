#![allow(unused)]
use crate::libs::gfx::two_d::{Point, Size, Rgba8888, Canvas2D, PrimitiveRect, Paint, Stroke, Drawable, CornerRadii};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use embassy_time::{Duration, Timer, Instant};
use micromath::F32Ext;

fn draw_rects(canvas: &mut Canvas2D, t: f32) {
    let w = canvas.width() as i32;
    let h = canvas.height() as i32;
    let center_x = w / 2;
    let center_y = h / 2;
    
    // Clear background
    canvas.clear_color(Rgba8888::new(30, 35, 45, 255));
    
    // Rectangle with ASYMMETRIC CORNER RADII + TRANSPARENT GRADIENT
    let rect_width = w * 2 / 3;
    let rect_height = h / 2;
    let rect_x = center_x - rect_width / 2;
    let rect_y = center_y - rect_height / 2;
    
    // ASYMMETRIC corner radii - each corner different but reasonable
    let anim = (t * 0.5).sin() * 0.5 + 0.5;
    let top_left = 5.0 + anim * 10.0;      // Small: 5-15px
    let top_right = 25.0 + anim * 15.0;    // Large: 25-40px  
    let bottom_right = 10.0 + anim * 10.0; // Medium: 10-20px
    let bottom_left = 35.0 + anim * 10.0;  // Very Large: 35-45px
    
    // Transparent gradient from pink to blue
    let gradient = Paint::linear(
        Point::new(rect_x, rect_y),
        Point::new(rect_x + rect_width, rect_y + rect_height),
        Rgba8888::new(255, 120, 180, 160),  // Semi-transparent pink
        Rgba8888::new(120, 180, 255, 200),  // Semi-transparent blue
    );
    
    // Draw rectangle with DIFFERENT corner radii for each corner
    PrimitiveRect::new(
        Point::new(rect_x, rect_y),
        Size::new(rect_width as u32, rect_height as u32)
    )
    .fill(gradient)
    .corner_radii(CornerRadii::new(top_left, top_right, bottom_right, bottom_left))
    .stroke(Stroke::new(Rgba8888::new(255, 255, 255, 255), 4.0))
    .draw(canvas);
    
    // No debug spam - just clean rendering
}

#[embassy_executor::task]
pub async fn rect_app(context: AppContext) {
    let start = Instant::now();
    
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        
        let t = start.elapsed().as_micros() as f32 / 1_000_000.0;
        
        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            // Use Canvas2D fluent drawing over the DrawingSurface
            let mut c2d = Canvas2D::new(surface as &mut dyn crate::libs::gfx::two_d::Rasterizer);
            draw_rects(&mut c2d, t);
            
            let draw_duration = draw_start.elapsed();
            if draw_duration.as_millis() > 5 {
                defmt::info!("Rect draw: {}ms", draw_duration.as_millis());
            }
        }).await;

        Timer::after(Duration::from_millis(16)).await; // 60fps
    }
}
