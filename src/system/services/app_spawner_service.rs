use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::apps::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;
use crate::tasks::battery::battery_task;

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {
    // === APP 1: Battery ===
    // Lock just long enough to get window+canvas
    let (handle, canvas) = {
        let mut comp = compositor.lock().await;
        comp.alloc_window_with_canvas(128, 64, 0).await
    }.expect("Failed to allocate battery window");

    let ctx = AppContext::new(
        handle,
        canvas,
        0,
        "Battery",
        compositor, // 👈 Pass original static ref
    );

    spawner.spawn(battery_task(ctx)).unwrap();
}
