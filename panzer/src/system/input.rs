//! Input Event System
//!
//! Defines input events for touch, buttons, and keyboard.

/// Input events that can be sent to applications
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// Touch event with coordinates and press state
    Touch {
        x: u16,
        y: u16,
        pressed: bool,
    },
    
    /// Button event with ID and press state
    Button {
        id: u8,
        pressed: bool,
    },
    
    /// Keyboard event with character
    Keyboard {
        key: char,
    },
}

impl InputEvent {
    /// Create a new touch event
    pub fn touch(x: u16, y: u16, pressed: bool) -> Self {
        Self::Touch { x, y, pressed }
    }

    /// Create a new button press event
    pub fn button_press(id: u8) -> Self {
        Self::Button { id, pressed: true }
    }

    /// Create a new button release event
    pub fn button_release(id: u8) -> Self {
        Self::Button { id, pressed: false }
    }

    /// Create a new keyboard event
    pub fn keyboard(key: char) -> Self {
        Self::Keyboard { key }
    }
}

/// Focus events for app focus changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusEvent {
    /// App gained focus
    Gained,
    /// App lost focus
    Lost,
}

/// Lifecycle events for app state changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum LifecycleEvent {
    /// App was created
    Created,
    /// App window became visible
    Visible,
    /// App window was hidden but app still active
    Hidden,
    /// App was suspended (not updating)
    Suspended,
    /// App was resumed from suspension
    Resumed,
    /// App is being destroyed
    Destroyed,
}
