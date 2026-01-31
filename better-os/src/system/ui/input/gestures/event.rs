use crate::system::input::types::{PointerSample, TouchAction};

/// Pointer-oriented sample passed from the input pipeline into gesture recognizers.
#[derive(Clone, Copy, Debug)]
pub struct PointerEvent {
    pub pointer: PointerSample,
    pub action: TouchAction,
}

impl PointerEvent {
    /// Construct a new pointer event for gesture processing.
    pub const fn new(pointer: PointerSample, action: TouchAction) -> Self {
        Self { pointer, action }
    }
}
