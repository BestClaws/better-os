use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_executor::Spawner;

use crate::system::apps::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;
use crate::tasks::battery::battery_task;

/// Spawns all UI apps at boot with their AppContext.
#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {
    let mut app_id_counter = 0;

    // === Battery App ===
    {
        let app_id = app_id_counter;
        app_id_counter += 1;

        // Lock compositor, allocate window and *move out* of the lock
        let (handle, canvas) = {
            let mut comp = compositor.lock().await;
            comp.alloc_window_with_canvas(128, 64, app_id)
                .await
                .expect("Failed to allocate battery window")
        };

        let ctx = AppContext::new(handle, canvas, app_id, "battery");
        spawner.spawn(battery_task(ctx)).unwrap();
    }
}
