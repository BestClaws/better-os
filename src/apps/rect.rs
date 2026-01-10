//! Simplified test app using new gfx API

use crate::libs::gfx::{Color, Layer, Opacity, Point, Rect};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::info;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn rect_app(ctx: AppContext) {
    info!("Starting rect app (new gfx API)");
    
    let mut frame = 0u32;
    
    loop {
        if !ctx.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        
        ctx.draw(|surface: &mut DrawingSurface| {
            // Create layer from surface
            let mut layer = Layer::from_draw_target(surface);
            
            // Clear background to black
            layer.clear(Color::BLACK);
            
            // Draw filled rectangles with different colors
            layer.fill(Rect::new(10, 10, 50, 50))
                .color(Color::RED)
                .draw();
            
            layer.fill(Rect::new(70, 10, 50, 50))
                .color(Color::GREEN)
                .draw();
            
            layer.fill(Rect::new(130, 10, 50, 50))
                .color(Color::BLUE)
                .draw();
            
            // Draw semi-transparent rectangles
            layer.fill(Rect::new(10, 70, 50, 50))
                .color(Color::YELLOW)
                .opacity(Opacity::OPA_50)
                .draw();
            
            layer.fill(Rect::new(70, 70, 50, 50))
                .color(Color::CYAN)
                .opacity(Opacity::OPA_70)
                .draw();
            
            layer.fill(Rect::new(130, 70, 50, 50))
                .color(Color::MAGENTA)
                .opacity(Opacity::OPA_30)
                .draw();
            
            // Animated rectangle
            let anim_x = 10 + ((frame / 2) % 170) as i32;
            layer.fill(Rect::new(anim_x, 130, 40, 40))
                .color(Color::WHITE)
                .draw();
        })
        .await;
        
        frame = frame.wrapping_add(1);
        Timer::after(Duration::from_millis(16)).await;
    }
}
