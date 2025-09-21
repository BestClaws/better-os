#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use crate::libs::gfx::two_d::{
    Point as GPoint, Size as GSize, Rect as GRect,
    Rgba8888, Canvas2D, FluentRect, Paint, Stroke
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::drawing_surface::DrawingSurface;
use micromath::F32Ext;

#[embassy_executor::task]
pub async fn demo_2d_app(context: AppContext) {
    info!("demo_2d app started");
    let start = Instant::now();
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        let t = start.elapsed().as_micros() as f32 / 1_000_000.0; // seconds
        let scene = ((t as u32) / 5) % 3; // switch every 5 seconds

        context.draw(|surface: &mut DrawingSurface| {
            let draw_start = Instant::now();
            let w = surface.width() as i32; 
            let h = surface.height() as i32;
            let mut c2d: Canvas2D = Canvas2D::new(surface as &mut dyn crate::libs::gfx::two_d::Rasterizer);

            // Background base per scene
            let sweep = ((t * 0.2).sin() * 0.5 + 0.5) as f32;
            let bg = Rgba8888::new(
                (20.0 + 120.0 * sweep) as u8,
                (20.0 + 80.0 * (1.0 - sweep)) as u8,
                (40.0 + 100.0 * sweep) as u8,
                255,
            );
            {
                // Using new fluent API instead of old Draw
                FluentRect::new(GRect::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32)))
                    .fill_rgba(bg)
                    .draw();
            }

            match scene {
                0 => {
                    // Animated rounded rectangles
                    let rect_w = (w as f32 * (0.25 + 0.05 * (t * 1.1).sin())) as u32;
                    let rect_h = (h as f32 * (0.18 + 0.05 * (t * 1.3).cos())) as u32;
                    let x = (w - rect_w as i32) / 2 + (w as f32 * 0.15 * (t * 0.7).sin()) as i32;
                    let y = (h - rect_h as i32) / 2 + (h as f32 * 0.10 * (t * 0.9).cos()) as i32;
                    {
                        // Using new fluent API instead of old Draw
                        FluentRect::new(GRect::new(GPoint::new(x, y), GSize::new(rect_w, rect_h)))
                            .corner_radius((8.0 + 6.0 * (t * 1.7).sin().abs()) as i32)
                            .fill_rgba(Rgba8888::new(80, 180, 240, 255))
                            // .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0))
                            .draw();
                    }
                }
                1 => {
                    // Simplified solid color rectangles instead of expensive gradients
                    let ur = GRect::new(GPoint::new(w / 4, h / 6), GSize::new((w / 2) as u32, (h / 5) as u32));
                    {
                        // Using new fluent API instead of old Draw
                        FluentRect::new(ur)
                            .corner_radius(10)
                            .fill_rgba(Rgba8888::new(255, 80, 80, 255))
                            // .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0))
                            .draw();
                    }

                    // Simplified solid color rect with non-uniform corners
                    let radii = 6.0; // corner radius
                    let nr = GRect::new(GPoint::new(w / 6, h / 2), GSize::new((w * 2 / 3) as u32, (h / 3) as u32));
                    {
                        // Using new fluent API instead of old Draw
                        FluentRect::new(nr)
                            .corner_radii(radii)
                            .fill_rgba(Rgba8888::new(40, 200, 120, 255))
                            // .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0))
                            .draw();
                    }
                }
                _ => {
                    // Arcs and lines scene
                    let c = GPoint::new(w / 2, h / 2);
                    let r1 = (h.min(w) / 3) as i32;
                    {
                        // Using new fluent API instead of old Draw
                        // Arc::new(c, r1, t * 1.2, t * 1.2 + 1.9).color(crate::libs::gfx::two_d::Rgba8888::opaque(255, 220, 80)).draw();
                    }
                    {
                        // Using new fluent API instead of old Draw
                        // Arc::new(c, r1 - 14, -t * 1.4, -t * 1.4 + 1.2).color(crate::libs::gfx::two_d::Rgba8888::opaque(80, 255, 200)).draw();
                    }

                    let lx = (w as f32 * (0.5 + 0.4 * (t * 0.8).sin())) as i32;
                    {
                        // Using new fluent API instead of old Draw
                        // Line::new(GPoint::new(12, 12), GPoint::new(lx, h - 12))
                            .color(crate::libs::gfx::two_d::Rgba8888::opaque(240, 240, 240))
                            .draw();
                    }
                }
            }

            // Non-uniform corner radii demo (animated per-corner)
            let nu_rect_w = (w as f32 * 0.38) as u32;
            let nu_rect_h = (h as f32 * 0.22) as u32;
            let nu_x = (w - nu_rect_w as i32) / 2;
            let nu_y = (h / 2 + 10).min(h - nu_rect_h as i32 - 6);
            let r_t = (6.0 + 5.0 * (t * 1.1).sin().abs()) as i32;
            let r_r = (10.0 + 7.0 * (t * 0.9).cos().abs()) as i32;
            let r_b = (4.0 + 6.0 * (t * 1.5).sin().abs()) as i32;
            let r_l = (12.0 + 5.0 * (t * 0.7).cos().abs()) as i32;
            let radii = r_t; // corner radius
            {
                // Using new fluent API instead of old Draw
                FluentRect::new(GRect::new(GPoint::new(nu_x, nu_y), GSize::new(nu_rect_w, nu_rect_h)))
                    .corner_radii(radii)
                    .fill_rgba(Rgba8888::new(60, 120, 220, 210))
                    // .stroke(Stroke::new(Rgba8888::opaque(255, 255, 255), 1.0))
                    .draw();
            }

            // RGBA overlay pulse
            let pulse = (0.5 + 0.5 * (t * 2.0).sin()).max(0.0).min(1.0);
            let alpha = (40.0 + 140.0 * pulse) as u8;
            {
                // Using new fluent API instead of old Draw
                FluentRect::new(GRect::new(GPoint::new(6, 6), GSize::new((w - 12) as u32, (h / 6) as u32)))
                    .fill_rgba(Rgba8888::new(255, 255, 255, alpha))
                    .draw();
            }

            // Arcs clock-like motion
            let c = GPoint::new(w / 2, h / 2);
            let r1 = (h.min(w) / 3) as i32;
            {
                // Using new fluent API instead of old Draw
                // Arc::new(c, r1, t * 1.2, t * 1.2 + 1.9).color(crate::libs::gfx::two_d::Rgba8888::opaque(255, 220, 80)).draw();
            }
            {
                // Using new fluent API instead of old Draw
                // Arc::new(c, r1 - 14, -t * 1.4, -t * 1.4 + 1.2).color(crate::libs::gfx::two_d::Rgba8888::opaque(80, 255, 200)).draw();
            }

            // Lines crossfade
            let lx = (w as f32 * (0.5 + 0.4 * (t * 0.8).sin())) as i32;
            {
                // Using new fluent API instead of old Draw
                // Line::new(GPoint::new(12, 12), GPoint::new(lx, h - 12))
                    .color(crate::libs::gfx::two_d::Rgba8888::opaque(240, 240, 240))
                    .draw();
            }

            // Moving translucent overlay highlight
            let rx = (w as f32 * (0.5 + 0.3 * (t * 0.6).cos())) as i32;
            let ry = (h as f32 * (0.5 + 0.3 * (t * 0.7).sin())) as i32;
            let overlay = Rgba8888::new(255, 255, 255, 28);
            {
                // Using new fluent API instead of old Draw
                FluentRect::new(GRect::new(GPoint::new(rx - (w / 4), ry - (h / 8)), GSize::new((w / 2) as u32, (h / 4) as u32)))
                    .fill_rgba(overlay)
                    .draw();
            }
            
            let draw_duration = draw_start.elapsed();
            if draw_duration.as_millis() > 30 {
                defmt::info!("Demo2D draw slow: {}ms, scene={}", draw_duration.as_millis(), scene);
            }
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(50)).await; // ~20 FPS to reduce I2C contention
    }
}


