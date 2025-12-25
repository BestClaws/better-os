use defmt::trace;
use embassy_executor::task;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

use crate::system::input::{bus, router::InputRouter};
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;

/// Pulls normalized input events from the shared bus and routes them to either the
/// System UI or the focused application window.
#[task]
pub async fn input_dispatcher_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    let router = InputRouter::new(compositor, window_manager);

    loop {
        let event = bus::raw_events().receive().await;
        trace!("dispatching input event");
        router.dispatch(event).await;
    }
}
