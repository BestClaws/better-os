use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

use crate::system::input::types::HighLevelEvent;

pub const RAW_EVENT_CAPACITY: usize = 64;
pub const SYSTEM_UI_EVENT_CAPACITY: usize = 64;
pub const SYSTEM_UI_ACK_CAPACITY: usize = 64;

static RAW_EVENT_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, RAW_EVENT_CAPACITY> =
    Channel::new();
static SYSTEM_UI_EVENT_CH: Channel<
    CriticalSectionRawMutex,
    HighLevelEvent,
    SYSTEM_UI_EVENT_CAPACITY,
> = Channel::new();
static SYSTEM_UI_ACK_CH: Channel<CriticalSectionRawMutex, bool, SYSTEM_UI_ACK_CAPACITY> =
    Channel::new();

/// Channel carrying debounced, high-level input events produced by hardware readers.
#[inline]
pub fn raw_events() -> &'static Channel<CriticalSectionRawMutex, HighLevelEvent, RAW_EVENT_CAPACITY>
{
    &RAW_EVENT_CH
}

/// Channel delivering events from the dispatcher to the System UI gesture processor.
#[inline]
pub fn system_ui_events(
) -> &'static Channel<CriticalSectionRawMutex, HighLevelEvent, SYSTEM_UI_EVENT_CAPACITY> {
    &SYSTEM_UI_EVENT_CH
}

/// Channel used by the System UI to acknowledge whether it consumed an event.
#[inline]
pub fn system_ui_acknowledgements(
) -> &'static Channel<CriticalSectionRawMutex, bool, SYSTEM_UI_ACK_CAPACITY> {
    &SYSTEM_UI_ACK_CH
}
