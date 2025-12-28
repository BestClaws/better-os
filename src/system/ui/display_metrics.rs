use crate::system::hal::display::DisplayResolution;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayMetrics {
    pub logical_width: u32,
    pub logical_height: u32,
    pub scale: u32,
}

impl DisplayMetrics {
    pub const fn new(logical_width: u32, logical_height: u32, scale: u32) -> Self {
        Self {
            logical_width,
            logical_height,
            scale,
        }
    }
}

static LOGICAL_WIDTH: AtomicU32 = AtomicU32::new(0);
static LOGICAL_HEIGHT: AtomicU32 = AtomicU32::new(0);
static SCALE: AtomicU32 = AtomicU32::new(1);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn update(resolution: DisplayResolution) {
    LOGICAL_WIDTH.store(resolution.logical.width, Ordering::Relaxed);
    LOGICAL_HEIGHT.store(resolution.logical.height, Ordering::Relaxed);
    SCALE.store(resolution.scale.max(1), Ordering::Relaxed);
    INITIALIZED.store(true, Ordering::Release);
}

pub fn set(logical_width: u32, logical_height: u32, scale: u32) {
    LOGICAL_WIDTH.store(logical_width, Ordering::Relaxed);
    LOGICAL_HEIGHT.store(logical_height, Ordering::Relaxed);
    SCALE.store(scale.max(1), Ordering::Relaxed);
    INITIALIZED.store(true, Ordering::Release);
}

pub fn metrics() -> Option<DisplayMetrics> {
    if !INITIALIZED.load(Ordering::Acquire) {
        return None;
    }
    Some(DisplayMetrics::new(
        LOGICAL_WIDTH.load(Ordering::Relaxed),
        LOGICAL_HEIGHT.load(Ordering::Relaxed),
        SCALE.load(Ordering::Relaxed).max(1),
    ))
}

pub fn logical_size() -> Option<(u32, u32)> {
    metrics().map(|m| (m.logical_width, m.logical_height))
}

pub fn scale_factor() -> Option<u32> {
    metrics().map(|m| m.scale)
}
