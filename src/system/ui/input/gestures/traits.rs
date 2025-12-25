use super::{event::PointerEvent, result::GestureResult};

/// Common behaviour for pointer-driven gesture recognizers.
pub trait PointerGesture {
    type Update;

    /// Static identifier useful for debugging and metrics.
    fn name(&self) -> &'static str;

    /// Observe the next pointer event and optionally emit an update.
    fn observe(&mut self, event: PointerEvent) -> GestureResult<Self::Update>;

    /// Reset the internal state machine.
    fn reset(&mut self);

    /// Whether the recognizer is actively tracking a gesture.
    fn is_tracking(&self) -> bool {
        false
    }
}
