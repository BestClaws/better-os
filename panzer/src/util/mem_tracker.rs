//! Memory allocation tracking utility
//!
//! Provides RAII-based tracking of temporary memory allocations to monitor
//! peak memory usage during operations. Useful for profiling and optimization.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Current total of tracked allocations
static ALLOCATION_CURRENT: AtomicUsize = AtomicUsize::new(0);

/// Peak tracked allocation reached
static ALLOCATION_PEAK: AtomicUsize = AtomicUsize::new(0);

/// RAII guard that tracks memory allocation and automatically decrements on drop
///
/// # Example
/// ```
/// let data = vec![0u8; 1024];
/// let _guard = MemTracker::track(data.capacity());
/// // Memory is tracked here
/// // Guard drops automatically, decrementing the counter
/// ```
pub struct MemTracker {
    bytes: usize,
}

impl MemTracker {
    /// Start tracking an allocation of the given size in bytes
    ///
    /// Returns a guard that will automatically decrement the counter when dropped.
    #[inline]
    pub fn track(bytes: usize) -> Self {
        if bytes == 0 {
            return Self { bytes: 0 };
        }
        increase_allocation(bytes);
        Self { bytes }
    }

    /// Update the tracked allocation size
    ///
    /// Useful when a vector is resized or capacity changes.
    #[inline]
    pub fn update(&mut self, new_bytes: usize) {
        if new_bytes == self.bytes {
            return;
        }
        if new_bytes > self.bytes {
            let delta = new_bytes - self.bytes;
            increase_allocation(delta);
        } else {
            let delta = self.bytes - new_bytes;
            decrease_allocation(delta);
        }
        self.bytes = new_bytes;
    }
}

impl Drop for MemTracker {
    fn drop(&mut self) {
        decrease_allocation(self.bytes);
        self.bytes = 0;
    }
}

#[inline]
fn increase_allocation(bytes: usize) {
    if bytes == 0 {
        return;
    }
    let total = ALLOCATION_CURRENT.fetch_add(bytes, Ordering::Relaxed) + bytes;
    update_allocation_peak(total);
}

#[inline]
fn decrease_allocation(bytes: usize) {
    if bytes == 0 {
        return;
    }
    ALLOCATION_CURRENT.fetch_sub(bytes, Ordering::Relaxed);
}

#[inline]
fn update_allocation_peak(candidate: usize) {
    let mut peak = ALLOCATION_PEAK.load(Ordering::Relaxed);
    while candidate > peak {
        match ALLOCATION_PEAK.compare_exchange_weak(
            peak,
            candidate,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(updated) => peak = updated,
        }
    }
}

/// Get the peak memory usage tracked since last reset
pub fn peak_bytes() -> usize {
    ALLOCATION_PEAK.load(Ordering::Relaxed)
}

/// Get the current memory usage being tracked
pub fn current_bytes() -> usize {
    ALLOCATION_CURRENT.load(Ordering::Relaxed)
}

/// Reset the peak counter to zero
pub fn reset_peak() {
    ALLOCATION_PEAK.store(0, Ordering::Relaxed);
}
