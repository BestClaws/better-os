use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::system::app::app_context::AppContext;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window::WindowHandle;

use crate::apps::slab::{ slab_app};

#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    spawner: Spawner,
) {

            let mut comp = compositor.lock().await;
            // Step 1b: Allocate and get canvas
            let whandle = comp
                .alloc_window(128, 64, 0)
                .await
                .expect("Failed to allocate battery window");




    // === Step 2: Construct app context and spawn ===
    let ctx = AppContext::new(whandle, 1, "hello", compositor);
    spawner.spawn(slab_app(ctx)).unwrap();
}
