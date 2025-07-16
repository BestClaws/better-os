use portable_atomic::{AtomicU8, Ordering};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Sender, Receiver},
    semaphore::GreedySemaphore,
};
use embassy_sync::semaphore::Semaphore;
use crate::system::services::human_input_srv::HumanInputEvent;

pub const MAX_CHANNELS: usize = 8;
pub const CHANNEL_CAPACITY: usize = 16;

pub static INPUT_CHANNEL_POOL: InputChannelPool = InputChannelPool::new();

#[derive(Debug)]
pub struct InputChannelHandle {
    pub(crate) id: usize,
    _private: (),
}

impl Drop for InputChannelHandle {
    fn drop(&mut self) {
        INPUT_CHANNEL_POOL.release(self);
    }
}

pub struct InputChannelPool {
    // initialized: AtomicU8,
    status: AtomicU8,
    permits: GreedySemaphore<CriticalSectionRawMutex>,
    channels: [Channel<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>; MAX_CHANNELS],
}

impl InputChannelPool {
    pub const fn new() -> Self {
        const CHANNELS: Channel<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> =
            Channel::new();

        Self {
            // initialized: AtomicU8::new(0),
            status: AtomicU8::new(0),
            permits: GreedySemaphore::new(MAX_CHANNELS),
            channels: [CHANNELS; MAX_CHANNELS],
        }
    }

    // pub fn init(&self) {
    //     // Safe one-time init guard
    //     if self.initialized.swap(1, Ordering::AcqRel) == 1 {
    //         panic!("InputChannelPool already initialized");
    //     }
    //
    //     for chan in &self.channels {
    //         // This is not strictly needed, since Channels are const-initialized.
    //         // But if you ever make them dynamic, this loop is your friend.
    //         let _ = chan;
    //     }
    // }

    pub fn try_allocate(&self) -> Option<InputChannelHandle> {
        for id in 0..MAX_CHANNELS {
            let mask = 1 << id;
            let prev = self.status.fetch_or(mask, Ordering::AcqRel);

            if prev & mask == 0 {
                return Some(InputChannelHandle { id, _private: () });
            }
        }
        None
    }

    pub async fn allocate(&self) -> Option<InputChannelHandle> {
        self.permits.acquire(1).await.ok()?;
        loop {
            if let Some(h) = self.try_allocate() {
                return Some(h);
            }
            // Yield or wait — depends on scheduler
        }
    }

    pub fn release(&self, handle: &InputChannelHandle) {
        let mask = !(1 << handle.id);
        self.status.fetch_and(mask, Ordering::AcqRel);
        self.permits.release(1);
    }

    pub fn sender(&self, handle: &InputChannelHandle) ->  Sender<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        self.channels[handle.id].sender()
    }

    pub fn receiver(&self, handle: &InputChannelHandle) -> Receiver<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        self.channels[handle.id].receiver()
    }
}


