#![no_std]

use embassy_time::{Duration, Timer};
use defmt::info;
use crate::libs::gfx::two_d::{Point as GPoint, Size as GSize, Rect as GRect, Rgb565};
use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use crate::libs::ui::{Theme, ImUi, ImInput};

// retained-mode demo removed

#[embassy_executor::task]
pub async fn ui_demo_app(context: AppContext) {
    info!("UI demo app started");
    let theme = Theme::default();

    let mut counter: u32 = 0;
    loop {
        if !context.is_focused().await {
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        context.draw_immediate(|canvas: &mut Canvas, input: ImInput| {
            // Use immediate-mode UI with built-in input snapshot
            let mut ui = ImUi::new_fullscreen(canvas, input);
            ui.clear_background(Rgb565::from_rgb(15, 15, 18));


            // Simple counter demo
            // Safe here because single-threaded app task; if not, guard with Mutex
                if ui.add_button_horizontal("++++++").clicked() { counter += 1}

            use heapless::String;
            let mut s: String<32> = String::new();
            use core::fmt::Write;
            let _ = write!(&mut s, "{}t", counter);
            ui.add_label_newline(&s);


        }).await;

        context.request_redraw().await;
        Timer::after(Duration::from_millis(16)).await; // ~60 FPS
    }
}

fn fill_rect(canvas: &mut Canvas, rect: GRect, color: Rgb565) {
    use crate::libs::gfx::two_d::primitives::fill_rect as fill_rect_impl;
    fill_rect_impl(canvas, rect, color);
}

// retained-mode mapping removed

 
