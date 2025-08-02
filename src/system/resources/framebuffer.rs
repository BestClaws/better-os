#![allow(unused)]

use defmt::println;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use portable_atomic::{AtomicU8, Ordering};
use core::cell::UnsafeCell;
use embassy_time::Timer;
use crate::system::kernel::config::resources::{FRAME_BUFFER_COUNT, FRAME_BUFFER_SIZE};


/// Global singleton framebuffer pool.
pub static FRAMEBUFFER_POOL: FrameBufferPool = FrameBufferPool::new();







/// A handle uniquely representing an allocated framebuffer slot.
/// The handle is used to safely access a buffer in the pool,
/// ensuring exclusive access via controlled allocation.
#[derive(Debug)]
pub struct FrameBufferHandle {
    pub(crate) id: usize, // Index into the buffer pool
    _private: (),         // Prevent external construction (sealing)
}




impl FrameBufferHandle {
    /// Create a new handle for internal use.
    /// External code should only get handled via allocation.
    pub fn new(id: usize) -> Self {
        Self { id, _private: () }
    }
}




/// A static pool managing a fixed number of reusable framebuffers.
///
/// Buffers are not protected by a `Mutex` — instead, safety is ensured
/// by tracking allocation with `AtomicU8` and `GreedySemaphore`.
///
/// Each buffer is wrapped in `UnsafeCell` to allow interior mutability.
pub struct FrameBufferPool {
    // Array of raw buffers — one per slot, each with exclusive access guaranteed via handle
    buffers: [UnsafeCell<[u8; FRAME_BUFFER_SIZE]>; FRAME_BUFFER_COUNT],

    // Bitmap tracking which slots are in use (1 bit per buffer)
    status: AtomicU8,

    // Semaphore tracks how many free buffers are available
    permits: GreedySemaphore<CriticalSectionRawMutex>,
}

impl FrameBufferPool {
    /// Construct a new buffer pool with all buffers zero-initialized and free.
    pub const fn new() -> Self {
        Self {
            buffers: [ const { UnsafeCell::new([0; FRAME_BUFFER_SIZE]) }; FRAME_BUFFER_COUNT], // Safe because it's Copy + const init
            status: AtomicU8::new(0),                // All buffers initially free
            permits: GreedySemaphore::new(FRAME_BUFFER_COUNT), // All permits available
        }
    }

    /// Attempt to allocate a buffer slot non-blocking-ly.
    /// Returns `Some(FrameBufferHandle)` if successful, or `None` if all are taken.
    pub fn try_allocate(&self) -> Option<FrameBufferHandle> {
        for id in 0..FRAME_BUFFER_COUNT {
            let mask = 1 << id;
            let prev = self.status.fetch_or(mask, Ordering::AcqRel);

            // SAFETY: If the bit was previously 0, we just claimed it.
            if prev & mask == 0 {
                return Some(FrameBufferHandle::new(id));
            }
        }
        None
    }

    /// Allocate a buffer asynchronously, waiting until one becomes available.
    pub async fn allocate(&self) -> Option<FrameBufferHandle> {
        // Wait until a permit is available (non-blocking under async executor)
        self.permits.acquire(1).await.ok()?;
        loop {
            // Try to acquire an unused buffer
            if let Some(handle) = self.try_allocate() {
                return Some(handle);
            }
            Timer::after_micros(100).await;
        }
    }

    /// Release a previously allocated buffer slot.
    ///
    /// This marks the buffer as available and releases one permit.
    ///
    /// # Safety
    /// Caller must only release handles that were previously allocated
    /// and not still in use elsewhere.
    pub fn release(&self, handle: &FrameBufferHandle) {
        let mask = !(1 << handle.id);
        self.status.fetch_and(mask, Ordering::AcqRel); // Clear allocation bit
        self.permits.release(1); // Return permit to pool
    }

    /// Get a mutable reference to the buffer at `handle.id`.
    ///
    /// # Safety
    /// - The caller must ensure exclusive access to the buffer.
    /// - This is guaranteed by the pool’s allocation system, which issues a single valid
    ///   `FrameBufferHandle` per slot and prevents duplication or aliasing.
    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self, handle: &FrameBufferHandle) -> &mut [u8; FRAME_BUFFER_SIZE] {
        // SAFETY: Access is gated by handle ownership — only one valid mutable reference
        // should exist at any time, and `UnsafeCell` permits interior mutability.
        unsafe { &mut *self.buffers[handle.id].get() }
    }
}

// SAFETY: The pool is `Sync` because access is controlled via external mechanisms:
// - Buffers are only accessed via `get_mut()` using a valid handle.
// - Each buffer is exclusively owned after allocation.
// - Interior mutability via `UnsafeCell` is sound in this context.
unsafe impl Sync for FrameBufferPool {}

/// Drop implementation for handle — logs handle release but doesn't auto-return to pool.
///
/// Ideally, the pool would automatically reclaim the handle here, but static lifetime
/// makes access to the pool inside `Drop` non-trivial.
impl Drop for FrameBufferHandle {
    fn drop(&mut self) {
        FRAMEBUFFER_POOL.release(self);
        println!("Dropping FrameBufferHandle id={}", self.id);
    }
}
