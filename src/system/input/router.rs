use defmt::{debug, info, warn};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, WithTimeout};

use crate::system::input::{bus, types::HighLevelEvent};
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;

/// Handles the fan-out of high-level input events to the System UI and focused app windows.
pub struct InputRouter {
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    ack_timeout: Duration,
}

impl InputRouter {
    const DEFAULT_ACK_TIMEOUT_MS: u64 = 250;

    pub fn new(
        compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
        window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    ) -> Self {
        Self {
            compositor,
            window_manager,
            ack_timeout: Duration::from_millis(Self::DEFAULT_ACK_TIMEOUT_MS),
        }
    }

    /// Dispatch an event to the System UI first; if unconsumed, forward it to the focused window.
    pub async fn dispatch(&self, event: HighLevelEvent) {
        let dispatch_start = Instant::now();
        let (consumed, ack_duration) = self.deliver_to_system_ui(event).await;

        if consumed {
            debug!("System UI consumed event");
        } else {
            let forwarded = self.forward_to_focused_window(event).await;
            if !forwarded {
                warn!("No focused window to receive input");
            }
        }

        let total_duration = dispatch_start.elapsed();
        if total_duration.as_millis() > 50 || ack_duration.as_millis() > 30 {
            info!(
                "Input dispatch slow: total={}ms, ack={}ms, consumed={}",
                total_duration.as_millis(),
                ack_duration.as_millis(),
                consumed
            );
        }
    }

    async fn deliver_to_system_ui(&self, event: HighLevelEvent) -> (bool, Duration) {
        let ack_start = Instant::now();
        bus::system_ui_events().send(event).await;

        let ack_result = bus::system_ui_acknowledgements()
            .receive()
            .with_timeout(self.ack_timeout)
            .await;
        let ack_duration = ack_start.elapsed();

        match ack_result {
            Ok(consumed) => (consumed, ack_duration),
            Err(_) => {
                warn!(
                    "System UI ACK timeout after {}ms",
                    Self::DEFAULT_ACK_TIMEOUT_MS
                );
                (false, ack_duration)
            }
        }
    }

    async fn forward_to_focused_window(&self, event: HighLevelEvent) -> bool {
        let mut compositor = self.compositor.lock().await;
        let mut window_manager = self.window_manager.lock().await;

        if let Some(handle) = compositor.focused_window_handle() {
            let _ = window_manager.try_send_input(handle, event).await;
            compositor.request_redraw(handle);
            true
        } else {
            false
        }
    }
}
