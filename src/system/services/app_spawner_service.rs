use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::apps::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window::WindowHandle;
use crate::tasks::ambient::ambient_task;
use crate::tasks::battery::battery_task;
use crate::tasks::hello::hello_app;

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {
    // === Step 1: Allocate window and get canvas inside a separate scope ===
    let (handle, canvas): (WindowHandle, Canvas<'static>) = {
        let canvas = {
            // Step 1a: Lock the compositor
            let mut comp = compositor.lock().await;

            // Step 1b: Allocate and get canvas
            let (handle, canvas) = comp
                .alloc_window_with_canvas(128, 64, 0)
                .await
                .expect("Failed to allocate battery window");

            // Step 1c: SAFELY extend canvas lifetime to 'static
            let canvas: Canvas<'static> = unsafe { core::mem::transmute(canvas) };

            (handle, canvas)
        };

        canvas
    };

    // === Step 2: Construct app context and spawn ===
    let ctx = AppContext::new(handle, canvas, 0, "Battery", compositor);
    spawner.spawn(battery_task(ctx)).unwrap();




    // === Step 1: Allocate window and get canvas inside a separate scope ===
    let (handle, canvas): (WindowHandle, Canvas<'static>) = {
        let canvas = {
            // Step 1a: Lock the compositor
            let mut comp = compositor.lock().await;

            // Step 1b: Allocate and get canvas
            let (handle, canvas) = comp
                .alloc_window_with_canvas(128, 64, 2)
                .await
                .expect("Failed to allocate battery window");

            // Step 1c: SAFELY extend canvas lifetime to 'static
            let canvas: Canvas<'static> = unsafe { core::mem::transmute(canvas) };

            (handle, canvas)
        };

        canvas
    };

    // === Step 2: Construct app context and spawn ===
    let ctx = AppContext::new(handle, canvas, 0, "ambient", compositor);
    spawner.spawn(ambient_task(ctx)).unwrap();


    // === Step 1: Allocate window and get canvas inside a separate scope ===
    let (handle, canvas): (WindowHandle, Canvas<'static>) = {
        let canvas = {
            // Step 1a: Lock the compositor
            let mut comp = compositor.lock().await;

            // Step 1b: Allocate and get canvas
            let (handle, canvas) = comp
                .alloc_window_with_canvas(128, 64, 3)
                .await
                .expect("Failed to allocate battery window");

            // Step 1c: SAFELY extend canvas lifetime to 'static
            let canvas: Canvas<'static> = unsafe { core::mem::transmute(canvas) };

            (handle, canvas)
        };

        canvas
    };

    // === Step 2: Construct app context and spawn ===
    let ctx = AppContext::new(handle, canvas, 0, "hello", compositor);
    spawner.spawn(hello_app(ctx)).unwrap();
}
