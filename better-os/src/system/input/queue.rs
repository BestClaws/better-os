use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::system::input::types::HighLevelEvent;

/// Maximum number of pending input events retained in the raw hardware queue.
pub const RAW_EVENT_CAPACITY: usize = 64;

static RAW_EVENT_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, RAW_EVENT_CAPACITY> =
    Channel::new();

/// Lock-free queue ferrying debounced hardware events into the system.
///
/// The queue intentionally performs no gesture logic or routing; it merely
/// buffers normalized `HighLevelEvent`s produced by the device reader tasks.
/// Higher layers can pop events and decide whether to intercept them or
/// forward them to applications.
pub struct RawInputQueue;

impl RawInputQueue {
    /// Push a normalized event into the queue.
    #[inline]
    pub async fn push(event: HighLevelEvent) {
        RAW_EVENT_CH.send(event).await;
    }

    /// Pop the next event produced by the hardware readers.
    #[inline]
    pub async fn pop() -> HighLevelEvent {
        RAW_EVENT_CH.receive().await
    }

    /// Expose the backing channel when more advanced coordination is needed.
    #[inline]
    pub fn channel() -> &'static Channel<CriticalSectionRawMutex, HighLevelEvent, RAW_EVENT_CAPACITY>
    {
        &RAW_EVENT_CH
    }
}
