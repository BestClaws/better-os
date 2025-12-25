use crate::system::hal::encoder::EncoderState;
use crate::system::input::types::{HighLevelEvent, KeyAction, KeyCode, KeyEvent};

/// Map encoder rotation to the canonical confirmation key event.
#[inline]
pub fn map_state(state: EncoderState) -> HighLevelEvent {
    let _ = state; // Reserved for future directional mapping.
    HighLevelEvent::Key(KeyEvent {
        code: KeyCode::Ok,
        action: KeyAction::Down,
    })
}
