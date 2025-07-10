use portable_atomic::{AtomicU8, Ordering};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    semaphore::GreedySemaphore,
    channel::Channel,
    semaphore::Semaphore,
};
use static_cell::StaticCell;
use crate::system::services::human_input::HumanInputEvent;

pub const MAX_CHANNELS: usize = 8;
pub const CHANNEL_CAPACITY: usize = 16;

// Replace this with your real input event type

// StaticCell holding the array of channels (initialized once)
static CHANNEL_POOL_CELL: StaticCell<[Channel<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>; MAX_CHANNELS]> = StaticCell::new();

// Global mutable static Option storing the initialized channels reference
static mut CHANNELS: Option<&'static mut [Channel<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>; MAX_CHANNELS]> = None;

// Atomic bitmask tracking which channels are used (1 bit per channel)
static CHANNEL_USED: AtomicU8 = AtomicU8::new(0);

// Semaphore limiting concurrent channel allocations
pub static CHANNEL_SEMAPHORE: GreedySemaphore<CriticalSectionRawMutex> =
    GreedySemaphore::new(MAX_CHANNELS);

pub struct InputChannelHandle {
    id: usize,
    sender: embassy_sync::channel::Sender<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>,
    receiver: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>,
}

impl InputChannelHandle {
    pub fn sender(&self) -> &embassy_sync::channel::Sender<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        &self.sender
    }

    pub fn receiver(&self) -> &embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        &self.receiver
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

impl Drop for InputChannelHandle {
    fn drop(&mut self) {
        release_channel(self.id);
    }
}

/// Initialize the channel pool once. Must be called exactly once at startup.
pub fn init_channels() {
    let channels_ref = CHANNEL_POOL_CELL.init([
        Channel::new(),
        Channel::new(),
        Channel::new(),
        Channel::new(),
        Channel::new(),
        Channel::new(),
        Channel::new(),
        Channel::new(),
    ]);

    // SAFETY: Only call once and before any allocation.
    unsafe {
        CHANNELS = Some(channels_ref);
    }
}

/// Internal helper: Get mutable reference to the channel array.
/// SAFETY: Caller must ensure exclusive access (e.g. init called once before usage).
unsafe fn get_channels() -> &'static mut [Channel<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>; MAX_CHANNELS] {
    CHANNELS.as_mut().expect("Channels not initialized")
}

/// Try to allocate a free input channel. Returns None if all are used.
pub fn try_allocate_channel() -> Option<InputChannelHandle> {
    // SAFETY: We trust init_channels() was called before usage.
    let channels = unsafe { get_channels() };

    for id in 0..MAX_CHANNELS {
        let mask = 1 << id;
        let prev = CHANNEL_USED.fetch_or(mask, Ordering::AcqRel);

        if prev & mask == 0 {
            // Slot was free
            let channel = &channels[id];
            return Some(InputChannelHandle {
                id,
                sender: channel.sender(),
                receiver: channel.receiver(),
            });
        }
    }

    None
}

/// Async allocate a channel, waiting on semaphore if needed.
pub async fn allocate_channel() -> Option<InputChannelHandle> {
    CHANNEL_SEMAPHORE.acquire(1).await.ok()?;
    try_allocate_channel()
}

/// Release a channel slot back to the pool.
pub fn release_channel(id: usize) {
    let mask = !(1 << id);
    CHANNEL_USED.fetch_and(mask, Ordering::AcqRel);
    CHANNEL_SEMAPHORE.release(1);
}
