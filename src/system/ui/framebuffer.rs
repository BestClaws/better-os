#![allow(unused)]


use portable_atomic::{AtomicU8, Ordering};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    semaphore::GreedySemaphore,
};
use embassy_sync::semaphore::Semaphore;


pub const DISPLAY_WIDTH: usize = 128;
pub const DISPLAY_HEIGHT: usize = 64;
pub const FRAME_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT / 8;
pub const MAX_BUFFERS: usize = 8;

// === Global Pool ===

static mut FRAMEBUFFERS: [[u8; FRAME_SIZE]; MAX_BUFFERS] = [[0; FRAME_SIZE]; MAX_BUFFERS];
static BUFFER_USED: AtomicU8 = AtomicU8::new(0);
pub static FB_SEMAPHORE: GreedySemaphore<CriticalSectionRawMutex> =
    GreedySemaphore::new(MAX_BUFFERS);

// === Framebuffer Handle ===

pub struct FrameBufferHandle {
    id: usize,
    buffer: &'static mut [u8; FRAME_SIZE],
}

impl FrameBufferHandle {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..]
    }
}

impl Drop for FrameBufferHandle {
    fn drop(&mut self) {
        release_buffer(self.id);
    }
}

// === Public API ===

/// Try to allocate a framebuffer from pool.
/// Will return `None` if all are used.
pub fn try_allocate_buffer() -> Option<FrameBufferHandle> {
    for id in 0..MAX_BUFFERS {
        let mask = 1 << id;
        let prev = BUFFER_USED.fetch_or(mask, Ordering::AcqRel);

        if prev & mask == 0 {
            // This buffer was previously free
            unsafe {
                return Some(FrameBufferHandle {
                    id,
                    buffer: &mut FRAMEBUFFERS[id],
                });
            }
        }
    }

    None
}

/// Async wrapper over `try_allocate_buffer` using semaphore wait.
pub async fn allocate_buffer() -> Option<FrameBufferHandle> {
    FB_SEMAPHORE.acquire(1).await.ok()?;
    try_allocate_buffer()
}

/// Called internally or by compositor to release buffer.
pub fn release_buffer(id: usize) {
    let mask = !(1 << id);
    BUFFER_USED.fetch_and(mask, Ordering::AcqRel);
    FB_SEMAPHORE.release(1);
}

/// Get the underlying buffer data (read-only).
pub fn get_buffer_slice(id: usize) -> &'static [u8] {
    unsafe { &FRAMEBUFFERS[id] }
}
