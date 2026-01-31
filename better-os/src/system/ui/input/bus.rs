use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::system::input::types::HighLevelEvent;

/// Capacity of the System UI event queue.
pub const SYSTEM_UI_EVENT_CAPACITY: usize = 64;
/// Capacity of the acknowledgement queue mirroring each dispatched event.
pub const SYSTEM_UI_ACK_CAPACITY: usize = 64;

static SYSTEM_UI_EVENT_CH: Channel<
    CriticalSectionRawMutex,
    HighLevelEvent,
    SYSTEM_UI_EVENT_CAPACITY,
> = Channel::new();
static SYSTEM_UI_ACK_CH: Channel<CriticalSectionRawMutex, bool, SYSTEM_UI_ACK_CAPACITY> =
    Channel::new();

/// Shared event bus for the System UI to intercept hardware-originated input.
///
/// The dispatcher feeds events into [`SystemUiInputBus::events`], and the UI layer
/// consumes them and responds through [`SystemUiInputBus::acknowledgements`].
/// This keeps gesture recognition and input interception isolated from the
/// low-level device readers so each layer can evolve independently.
pub struct SystemUiInputBus;

impl SystemUiInputBus {
    /// Channel delivering input events to the System UI layer.
    #[inline]
    pub fn events(
    ) -> &'static Channel<CriticalSectionRawMutex, HighLevelEvent, SYSTEM_UI_EVENT_CAPACITY> {
        &SYSTEM_UI_EVENT_CH
    }

    /// Channel used by the System UI to acknowledge whether an event was consumed.
    #[inline]
    pub fn acknowledgements(
    ) -> &'static Channel<CriticalSectionRawMutex, bool, SYSTEM_UI_ACK_CAPACITY> {
        &SYSTEM_UI_ACK_CH
    }
}
