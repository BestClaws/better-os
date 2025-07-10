use embassy_executor::Spawner;
use crate::system::apps::context::{AppContext, AppMetadata};
use crate::system::ui::compositor::UICompositor;
use crate::system::apps::clock_app::clock_app; // Example app
use defmt::info;

static mut APP_COUNTER: usize = 0;

pub async fn app_spawner_service(spawner: Spawner, compositor: &mut UICompositor<'_>) {
    // Example app launch
    let width = 128;
    let height = 64;

    if let Some((handle, canvas)) = compositor.alloc_window(width, height).await {
        let app_id = unsafe {
            let id = APP_COUNTER;
            APP_COUNTER += 1;
            id
        };

        let metadata = AppMetadata {
            name: "Clock",
            size: (width as u32, height as u32),
        };

        let ctx = AppContext {
            app_id,
            window: handle,
            canvas,
            metadata,
        };

        spawner.spawn(clock_app(ctx)).unwrap();
        info!("Spawned app: Clock with ID {}", app_id);
    } else {
        defmt::warn!("Failed to allocate window for app");
    }
}
