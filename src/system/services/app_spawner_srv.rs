use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::apps::ble::ble_app;
use crate::apps::battery::battery_app;
use crate::apps::notifications::notifications_app;
use crate::system::app::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;

use crate::apps::slab::{ slab_app};

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {

    let mut comp = compositor.lock().await;
    let whandle = comp
    .alloc_window(128, 64, 0)
    .await
    .expect("Failed to allocate battery window");
    let ctx = AppContext::new(whandle, 0, "hello", compositor);
    spawner.spawn(slab_app(ctx)).unwrap();

    let whandle = comp
        .alloc_window(128, 64, 2)
        .await
        .expect("Failed to allocate battery window");
    let ctx = AppContext::new(whandle, 2, "battery", compositor);
    spawner.spawn(battery_app(ctx)).unwrap();

    let whandle = comp
        .alloc_window(128, 64, 1)
        .await
        .expect("Failed to allocate battery window");
    let ctx = AppContext::new(whandle, 1, "ble", compositor);
    spawner.spawn(ble_app(ctx)).unwrap();

    let whandle = comp
        .alloc_window(128, 64, 3)
        .await
        .expect("Failed to allocate vibration window");
    let ctx = AppContext::new(whandle, 3, "vibration", compositor);
    spawner.spawn(notifications_app(ctx)).unwrap();


}
