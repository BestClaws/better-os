#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use micromath::F32Ext;
use crate::libs::gfx::two_d::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565, TextRenderer, FONT_8X8};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;

/// Simple text demo application to showcase the text rendering system.
/// 
/// This app displays "HELLO WORLD 1 2 3" centered on the screen with
/// different colors and styles to demonstrate the text rendering capabilities.
#[embassy_executor::task]
pub async fn text_demo_app(context: AppContext) {
    info!("Text demo app started - showcasing space-grade text rendering");
    
    let start_time = Instant::now();
    
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }
        
        let t = start_time.elapsed().as_millis() as f32 / 1000.0;
        
        context.draw(|canvas: &mut Canvas| {
            let w = canvas.width() as i32;
            let h = canvas.height() as i32;
            
            // Clear background with black
            fill_rect(canvas, GRect::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32)), Rgb565::from_rgb(0, 0, 0));
            
            // Create simple text renderer with bright white
            let text_renderer = TextRenderer::new(&FONT_8X8)
                .with_color(Rgb565::from_rgb(255, 255, 255))
                .with_anti_alias(false); // Disable anti-aliasing for crisp text
            
            // Just draw "HELLO WORLD 1 2 3" centered
            let main_text = "HELLO WORLD 1 2 3";
            let text_width = text_renderer.measure_text(main_text);
            let text_height = FONT_8X8.info.line_height as i32;
            
            let center_x = (w - text_width) / 2;
            let center_y = (h - text_height) / 2;
            
            // Draw the main text
            text_renderer.draw_text(canvas, GPoint::new(center_x, center_y), main_text);
            
        }).await;
        
        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // ~60 FPS
    }
}

/// Helper function to fill a rectangle (reusing from existing graphics system)
fn fill_rect(canvas: &mut Canvas, rect: GRect, color: Rgb565) {
    use crate::libs::gfx::two_d::primitives::fill_rect;
    fill_rect(canvas, rect, color);
}
