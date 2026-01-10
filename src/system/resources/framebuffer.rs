use crate::system::hal::display::PixelFormat;
use crate::system::kernel::config::resources::MAX_FRAME_BUFFERS;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use defmt::println;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore};

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
    buffers: [UnsafeCell<Vec<u8>>; MAX_FRAME_BUFFERS],
    buffer_len: AtomicUsize,

    // Bitmap tracking which slots are in use (1 bit per buffer)
    status: AtomicU8,

    // Semaphore tracks how many free buffers are available
    permits: GreedySemaphore<CriticalSectionRawMutex>,
}

impl FrameBufferPool {
    /// Construct a new buffer pool with all buffers zero-initialized and free.
    pub const fn new() -> Self {
        Self {
            buffers: [const { UnsafeCell::new(Vec::new()) }; MAX_FRAME_BUFFERS],
            buffer_len: AtomicUsize::new(0),
            status: AtomicU8::new(0), // All buffers initially free
            permits: GreedySemaphore::new(MAX_FRAME_BUFFERS), // All permits available
        }
    }

    /// Configure buffers for the negotiated logical framebuffer layout.
    ///
    /// # Panics
    /// Panics if buffers are currently allocated or if the computed size overflows.
    pub fn configure(&self, width: u32, height: u32, format: PixelFormat) {
        let in_use = self.status.load(Ordering::Acquire);
        assert!(
            in_use == 0,
            "Cannot reconfigure framebuffer pool while buffers are allocated"
        );

        let required = format.framebuffer_size(width, height);
        assert!(required > 0, "Framebuffer dimensions must be non-zero");

        for buffer in self.buffers.iter() {
            // SAFETY: exclusive because configure is only called when no buffers are allocated.
            let vec = unsafe { &mut *buffer.get() };
            vec.resize(required, 0);
        }
        self.buffer_len.store(required, Ordering::Release);
    }

    /// Attempt to allocate a buffer slot non-blocking-ly.
    /// Returns `Some(FrameBufferHandle)` if successful, or `None` if all are taken.
    pub fn _try_allocate(&self) -> Option<FrameBufferHandle> {
        for id in 0..MAX_FRAME_BUFFERS {
            let mask = 1 << id;
            let prev = self.status.fetch_or(mask, Ordering::AcqRel);

            // SAFETY: If the bit was previously 0, we just claimed it.
            if prev & mask == 0 {
                return Some(FrameBufferHandle::new(id));
            }
        }
        None
    }

    /// try to allocate a buffer
    pub async fn allocate(&self) -> Option<FrameBufferHandle> {
        // Wait until a permit is available (non-blocking under async executor)
        self.permits.acquire(1).await.ok()?;
        // Try to acquire an unused buffer
        self._try_allocate()
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
    pub fn get_mut(&self, handle: &FrameBufferHandle) -> &mut [u8] {
        // SAFETY: Access is gated by handle ownership — only one valid mutable reference
        // should exist at any time, and `UnsafeCell` permits interior mutability.
        let len = self.buffer_len.load(Ordering::Acquire);
        assert!(len > 0, "FrameBufferPool not configured");
        unsafe {
            let vec = &mut *self.buffers[handle.id].get();
            if vec.len() < len {
                vec.resize(len, 0);
            }
            &mut vec[..len]
        }
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
