use embassy_executor::task;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use crate::system::input::pipeline::{EventPipeline, EventRouter};
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::windowing::WindowManager;

/// Pulls normalized input events from the shared bus and routes them to either the
/// System UI or the focused application window.
#[task]
pub async fn input_dispatcher_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    let router = EventRouter::new(compositor, window_manager);
    let pipeline = EventPipeline::new(router);
    pipeline.run().await;
}
