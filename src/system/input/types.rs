use defmt::Format;

#[derive(Clone, Copy, Debug, Format)]
pub enum KeyAction {
    Down,
    Up,
    Repeat,
    LongPress,
}

#[derive(Clone, Copy, Debug, Format)]
pub enum KeyCode {
    Ok,
}

#[derive(Clone, Copy, Debug, Format)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub action: KeyAction,
}

#[derive(Clone, Copy, Debug, Format)]
pub enum TouchAction {
    Down,
    Up,
    Move,
}

#[derive(Clone, Copy, Debug, Format)]
pub struct PointerSample {
    pub id: u8,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Format)]
pub struct MotionEvent {
    pub action: TouchAction,
    pub primary_pointer_id: u8,
    pub pointers: [Option<PointerSample>; 2],
}

#[derive(Clone, Copy, Debug, Format)]
pub enum HighLevelEvent {
    Key(KeyEvent),
    Motion(MotionEvent),
}


