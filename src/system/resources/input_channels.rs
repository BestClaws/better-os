use crate::system::input::types::HighLevelEvent;
use core::sync::atomic::{AtomicU8, Ordering};
use defmt::info;
use embassy_sync::semaphore::Semaphore;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
    semaphore::GreedySemaphore,
};
use embassy_time::Timer;

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
    status: AtomicU8,
    permits: GreedySemaphore<CriticalSectionRawMutex>,
    channels: [Channel<CriticalSectionRawMutex, HighLevelEvent, CHANNEL_CAPACITY>; MAX_CHANNELS],
}

impl InputChannelPool {
    pub const fn new() -> Self {
        Self {
            // initialized: AtomicU8::new(0),
            status: AtomicU8::new(0),
            permits: GreedySemaphore::new(MAX_CHANNELS),
            channels: [const { Channel::new() }; MAX_CHANNELS],
        }
    }

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
        self.try_allocate()
    }

    pub fn release(&self, handle: &InputChannelHandle) {
        let mask = !(1 << handle.id);
        self.status.fetch_and(mask, Ordering::AcqRel);
        self.permits.release(1);
    }

    pub fn sender(
        &self,
        handle: &InputChannelHandle,
    ) -> Sender<CriticalSectionRawMutex, HighLevelEvent, CHANNEL_CAPACITY> {
        self.channels[handle.id].sender()
    }

    pub fn receiver(
        &self,
        handle: &InputChannelHandle,
    ) -> Receiver<CriticalSectionRawMutex, HighLevelEvent, CHANNEL_CAPACITY> {
        self.channels[handle.id].receiver()
    }
}
