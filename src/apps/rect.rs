#![allow(unused)]
use crate::libs::gfx::color::Rgba8888;
use crate::libs::gfx::RoundedRect;
use crate::libs::gfx::shapes::Shape;
use crate::libs::gfx::rasterizer::Rasterizer;
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use embassy_time::{Duration, Instant, Timer};
use micromath::F32Ext;
use crate::util::math::primitives::{Point, Size};

fn draw_rects(surface: &mut DrawingSurface, t: f32) {
    let w = surface.width() as i32;
    let h = surface.height() as i32;
    let center_x = w / 2;
    let center_y = h / 2;
    // Clear background
    surface.fill_rect(0, 0, w, h, Rgba8888::rgba(30, 35, 45, 255));
    
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
    // Transparent gradient approximated via horizontal blend span
    let inner = Rgba8888::rgba(255, 120, 180, 160);
    let outer = Rgba8888::rgba(120, 180, 255, 200);

    let rr = RoundedRect::new(
        rect_x, rect_y,
        rect_width, rect_height,
        top_left as i32, top_right as i32, bottom_left as i32, bottom_right as i32,
    )
    .stroke(4, Rgba8888::rgba(255, 255, 255, 255))
    .fill_linear_h(inner, outer);
    rr.draw(surface);
    
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
            draw_rects(surface, t);
            
            let draw_duration = draw_start.elapsed();
            if draw_duration.as_millis() > 5 {
                defmt::info!("Rect draw: {}ms", draw_duration.as_millis());
            }
        }).await;

        Timer::after(Duration::from_millis(16)).await; // 60fps
    }
}
