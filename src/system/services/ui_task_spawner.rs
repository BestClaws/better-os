use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Instant;

use crate::system::ui::context::{AppId, AppMetadata, UIContext};
use crate::system::ui::manager::{allocate_frame, DISPLAY_WIDTH, DISPLAY_HEIGHT};
use crate::tasks::ambient::ambient_task;
use crate::tasks::battery::battery_task;

/// App IDs
const AMBIENT_APP_ID: AppId = 0;
const BATTERY_APP_ID: AppId = 1;

#[embassy_executor::task]
pub async fn ui_task_spawner(spawner: Spawner) {
    info!("[{}s] ui_task_spawner started", Instant::now().as_millis() as f32 / 1000f32);

    // Spawn ambient app
    if let Some(ctx) = create_context(AMBIENT_APP_ID).await {
        spawner.spawn(ambient_task(ctx)).unwrap();
    }

    // Spawn battery app
    if let Some(ctx) = create_context(BATTERY_APP_ID).await {
        spawner.spawn(battery_task(ctx)).unwrap();
    }
}

/// Helper to create a UIContext for a given app.
async fn create_context(app_id: AppId) -> Option<UIContext<'static>> {
    let frame = allocate_frame(app_id).await?;
    Some(UIContext {
        metadata: AppMetadata {
            app_id,
            display_width: DISPLAY_WIDTH as u32,
            display_height: DISPLAY_HEIGHT as u32,
        },
        canvas: frame.canvas,
    })
}
