use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use crate::system::ui::compositor::UICompositor;

static mut NEXT_APP_ID: usize = 0;

#[embassy_executor::task]
pub async fn app_spawner_service(
    spawner: Spawner) {}
