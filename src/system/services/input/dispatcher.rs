use defmt::trace;
use embassy_executor::task;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use crate::system::input::RawInputQueue;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::windowing::WindowManager;

use super::router::InputRouter;

/// Pulls normalized input events from the shared bus and routes them to either the
/// System UI or the focused application window.
#[task]
pub async fn input_dispatcher_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    let router = InputRouter::new(compositor, window_manager);

    loop {
        let event = RawInputQueue::pop().await;
        trace!("dispatching input event");
        router.dispatch(event).await;
    }
}
