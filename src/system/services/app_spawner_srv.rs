use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::apps::ble::ble_app;
use crate::apps::battery::battery_app;
use crate::apps::notifications::notifications_app;
use crate::system::app::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;

use crate::apps::arrow::{arrow_app};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {

    let mut comp = compositor.lock().await;


    let whandle = comp
        .alloc_window(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, 0)
        .await
        .expect("Failed to allocate battery window");
    let ctx = AppContext::new(whandle, 0, "hello", compositor);
    spawner.spawn(arrow_app(ctx)).unwrap();

    let whandle = comp
        .alloc_window(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, 1)
        .await
        .expect("Failed to allocate battery window");
    let ctx = AppContext::new(whandle, 1, "ble", compositor);
    spawner.spawn(ble_app(ctx)).unwrap();



}
