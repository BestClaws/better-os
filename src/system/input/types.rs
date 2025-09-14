use defmt::Format;

/// High-level input actions for keys/buttons.

#[derive(Clone, Copy, Debug, Format)]
pub enum KeyAction {
    Down,
    Up,
    Repeat,
    LongPress,
}

/// Logical key codes. Keep minimal on embedded.
#[derive(Clone, Copy, Debug, Format)]
pub enum KeyCode {
    Ok,
}

/// Discrete key event.
#[derive(Clone, Copy, Debug, Format)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub action: KeyAction,
}

/// Touch pointer state change.
#[derive(Clone, Copy, Debug, Format)]
pub enum TouchAction {
    Down,
    Up,
    Move,
}

/// A single pointer sample. Multi-touch reserved in second slot.
#[derive(Clone, Copy, Debug, Format)]
pub struct PointerSample {
    pub id: u8,
    pub x: i32,
    pub y: i32,
}

/// Motion event carrying up to two pointers. Primary is index 0.
#[derive(Clone, Copy, Debug, Format)]
pub struct MotionEvent {
    pub action: TouchAction,
    pub primary_pointer_id: u8,
    pub pointers: [Option<PointerSample>; 2],
}

/// High-level input event delivered to System UI and apps.
#[derive(Clone, Copy, Debug, Format)]
pub enum HighLevelEvent {
    Key(KeyEvent),
    Motion(MotionEvent),
}


