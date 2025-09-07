#![no_std]

use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use crate::libs::gfx::two_d::{
    Point as GPoint, Size as GSize, Rect as GRect,
    Rgba8888, FillStyle,
    EgRectangle, EgRoundedRectangle, EgLine, EgArc,
    EgPrimitiveStyleBuilder, EgDrawable, Canvas2D,
};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::DrawingSurface as Canvas;
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

        context.draw(|canvas: &mut Canvas| {
            let w = canvas.width() as i32; let h = canvas.height() as i32;
            let mut c2d: Canvas2D<crate::libs::gfx::two_d::Rgba8888> = Canvas2D::new(canvas as &mut dyn crate::libs::gfx::two_d::Rasterizer);

            // Background color sweep (RGBA)
            let sweep = ((t * 0.2).sin() * 0.5 + 0.5) as f32;
            let bg = Rgba8888::new(
                (20.0 + 120.0 * sweep) as u8,
                (20.0 + 80.0 * (1.0 - sweep)) as u8,
                (40.0 + 100.0 * sweep) as u8,
                255,
            );
            EgRectangle::new(GPoint::new(0, 0), GSize::new(w as u32, h as u32))
                .into_styled(EgPrimitiveStyleBuilder::new().fill_rgba(bg).build())
                .draw(&mut c2d);

            // Animated rounded rectangles
            let rect_w = (w as f32 * (0.25 + 0.05 * (t * 1.1).sin())) as u32;
            let rect_h = (h as f32 * (0.18 + 0.05 * (t * 1.3).cos())) as u32;
            let x = (w - rect_w as i32) / 2 + (w as f32 * 0.15 * (t * 0.7).sin()) as i32;
            let y = (h - rect_h as i32) / 2 + (h as f32 * 0.10 * (t * 0.9).cos()) as i32;
            EgRoundedRectangle::with_equal_corners(GRect::new(GPoint::new(x, y), GSize::new(rect_w, rect_h)), (8.0 + 6.0 * (t * 1.7).sin().abs()) as i32)
                .into_styled(EgPrimitiveStyleBuilder::new().fill_rgba(Rgba8888::new(80, 180, 240, 255)).stroke_width(1).stroke(Rgba8888::new(255, 255, 255, 255)).build())
                .draw(&mut c2d);

            // RGBA overlay pulse
            let pulse = (0.5 + 0.5 * (t * 2.0).sin()).max(0.0).min(1.0);
            let alpha = (40.0 + 140.0 * pulse) as u8;
            EgRectangle::new(GPoint::new(6, 6), GSize::new((w - 12) as u32, (h / 6) as u32))
                .into_styled(EgPrimitiveStyleBuilder::new().fill_rgba(Rgba8888::new(255, 255, 255, alpha)).build())
                .draw(&mut c2d);

            // Arcs clock-like motion
            let c = GPoint::new(w / 2, h / 2);
            let r1 = (h.min(w) / 3) as i32;
            EgArc::new(c, r1, t * 1.2, t * 1.2 + 1.9)
                .into_styled(EgPrimitiveStyleBuilder::new().stroke(Rgba8888::new(255, 220, 80, 255)).build())
                .draw(&mut c2d);
            EgArc::new(c, r1 - 14, -t * 1.4, -t * 1.4 + 1.2)
                .into_styled(EgPrimitiveStyleBuilder::new().stroke(Rgba8888::new(80, 255, 200, 255)).build())
                .draw(&mut c2d);

            // Lines crossfade
            let lx = (w as f32 * (0.5 + 0.4 * (t * 0.8).sin())) as i32;
            EgLine::new(GPoint::new(12, 12), GPoint::new(lx, h - 12))
                .into_styled(EgPrimitiveStyleBuilder::new().stroke(Rgba8888::new(240, 240, 240, 255)).build())
                .draw(&mut c2d);

            // Moving translucent overlay highlight
            let rx = (w as f32 * (0.5 + 0.3 * (t * 0.6).cos())) as i32;
            let ry = (h as f32 * (0.5 + 0.3 * (t * 0.7).sin())) as i32;
            let overlay = Rgba8888::new(255, 255, 255, 28);
            EgRectangle::new(GPoint::new(rx - (w / 4), ry - (h / 8)), GSize::new((w / 2) as u32, (h / 4) as u32))
                .into_styled(EgPrimitiveStyleBuilder::new().fill_rgba(overlay).build())
                .draw(&mut c2d);
        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // ~60 FPS
    }
}


