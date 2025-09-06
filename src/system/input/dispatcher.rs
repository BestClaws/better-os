use defmt::{debug, warn};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    mutex::Mutex,
};
use embassy_time::{Duration, Timer, WithTimeout};

use crate::system::input::reader::INPUT_EVENTS_CH;
use crate::system::input::types::HighLevelEvent;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;

/// ID -> SUI event queue
pub static SUI_EVENT_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();
/// SUI -> ID ack queue (true = consumed)
pub static SUI_ACK_CH: Channel<CriticalSectionRawMutex, bool, 64> = Channel::new();

/// Input dispatcher: routes events to System UI first; if rejected, forwards to focused app.
#[embassy_executor::task]
pub async fn input_dispatcher(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    loop {
        let event = INPUT_EVENTS_CH.receive().await;

        // Send to SUI
        SUI_EVENT_CH.send(event).await;

        // Wait for ACK with timeout
        let consumed = match SUI_ACK_CH
            .receive()
            .with_timeout(Duration::from_millis(100))
            .await {
            Ok(val) => val,
            Err(_) => false,
        };

        if consumed {
            debug!("SUI consumed event");
            continue;
        }

        // Forward to focused app
        let mut comp = compositor.lock().await;
        let mut wm = window_manager.lock().await;
        if let Some(handle) = comp.focused_window_handle() {
            let _ = wm.try_send_input(handle, event).await;
            // Hint compositor to redraw after app input
            comp.request_redraw(handle);
        } else {
            warn!("No focused window to receive input");
        }
    }
}


