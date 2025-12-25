use crate::system::hal::button::ButtonState;
use crate::system::input::types::{HighLevelEvent, KeyAction, KeyCode, KeyEvent};

/// Convert a `ButtonState` reported by the HAL into a high-level input event.
#[inline]
pub fn map_state(state: ButtonState) -> HighLevelEvent {
    let action = match state {
        ButtonState::Down => KeyAction::Down,
        ButtonState::Up => KeyAction::Up,
        ButtonState::Repeat => KeyAction::Repeat,
    };

    HighLevelEvent::Key(KeyEvent {
        code: KeyCode::Ok,
        action,
    })
}
