use crate::system::app::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::apps::battery::battery_app;
use crate::apps::dummy::dummy_app;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {

    // let ctx = create_context(spawner, compositor, 0, "arrow", 0).await;
    // spawner.spawn(arrow_app(ctx)).unwrap();
    // let ctx = create_context(spawner, compositor, 1, "ble", 1).await;
    // spawner.spawn(ble_app(ctx)).unwrap();
    // let ctx = create_context(spawner, compositor, 2, "notification", 2).await;
    // spawner.spawn(notifications_app(ctx)).unwrap();
    // let ctx = create_context(spawner, compositor, 3, "dummy", 3).await;
    // spawner.spawn(dummy_app(ctx)).unwrap();
    let ctx = create_context(spawner, compositor, 3, "battery", 3).await;
    spawner.spawn(battery_app(ctx)).unwrap();


}


pub async fn create_context(spawner: Spawner, compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>, id: usize, name: &'static str, app: i32) -> AppContext {
    let mut comp = compositor.lock().await;

    let whandle = comp
    .new_window(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, id)
    .await
    .expect("Failed to allocate app window");
     AppContext::new(whandle, id, name, compositor)
}
