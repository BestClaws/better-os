use alloc::boxed::Box;
use defmt::{debug, info, warn};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
    mutex::Mutex,
};
use embassy_time::{Duration, Timer};

use crate::system::hal::button::{AsyncButton, ButtonState};
use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
use crate::system::hal::touch::AsyncTouch;
use crate::system::input::types::{HighLevelEvent, KeyAction, KeyCode, KeyEvent, MotionEvent, PointerSample, TouchAction};
use crate::system::kernel::config::resources::{FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, FRAME_SCALE_FACTOR};

/// IR -> ID queue of high-level events (debounced/fused)
pub static INPUT_EVENTS_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();

#[embassy_executor::task]
pub async fn read_button(button: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>) {
    loop {
        let state = {
            let mut b = button.lock().await;
            b.next().await
        };

        let event = match state {
            ButtonState::Down => HighLevelEvent::Key(KeyEvent { code: KeyCode::Ok, action: KeyAction::Down }),
            ButtonState::Up => HighLevelEvent::Key(KeyEvent { code: KeyCode::Ok, action: KeyAction::Up }),
            ButtonState::Repeat => HighLevelEvent::Key(KeyEvent { code: KeyCode::Ok, action: KeyAction::Repeat }),
        };

        INPUT_EVENTS_CH.send(event).await;
    }
}

#[embassy_executor::task]
pub async fn read_encoder(encoder: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>) {
    loop {
        let Ok(state) = ({
            let mut e = encoder.lock().await;
            e.next().await
        }) else {
            warn!("Failed to read encoder state");
            continue;
        };

        let event = match state {
            // Map both directions to OK Down to keep single-button semantics minimal
            EncoderState::Ccw => HighLevelEvent::Key(KeyEvent { code: KeyCode::Ok, action: KeyAction::Down }),
            EncoderState::Cw => HighLevelEvent::Key(KeyEvent { code: KeyCode::Ok, action: KeyAction::Down }),
        };
        INPUT_EVENTS_CH.send(event).await;
        Timer::after(Duration::from_millis(200)).await; // cooldown
    }
}

/// Simple touch fuser: converts raw xyz polling into MotionEvent DOWN/MOVE/UP
#[embassy_executor::task]
pub async fn read_touch(touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>) {
    let mut was_pressed = false;
    let mut last_x: i32 = 0;
    let mut last_y: i32 = 0;

    loop {
        let (x, y, z) = {
            let mut t = touch.lock().await;
            t.read_xyz().await
        };

        let pressed = z != 0;
        // Normalize raw touch to framebuffer coordinates to match SUI edge thresholds
        let mut xi = (x as u32 / FRAME_SCALE_FACTOR) as i32;
        let mut yi = (y as u32 / FRAME_SCALE_FACTOR) as i32;
        // Clamp within framebuffer bounds
        if xi < 0 { xi = 0; }
        if yi < 0 { yi = 0; }
        if xi >= FRAME_BUFFER_WIDTH as i32 { xi = FRAME_BUFFER_WIDTH as i32 - 1; }
        if yi >= FRAME_BUFFER_HEIGHT as i32 { yi = FRAME_BUFFER_HEIGHT as i32 - 1; }

        let hle = if pressed && !was_pressed {
            // DOWN
            Some(HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Down,
                primary_pointer_id: 0,
                pointers: [Some(PointerSample { id: 0, x: xi, y: yi }), None],
            }))
        } else if pressed && was_pressed {
            // MOVEs - coalesce by distance to reduce spam
            let dx = (xi - last_x).abs();
            let dy = (yi - last_y).abs();
            if dx + dy >= 1 {
                Some(HighLevelEvent::Motion(MotionEvent {
                    action: TouchAction::Move,
                    primary_pointer_id: 0,
                    pointers: [Some(PointerSample { id: 0, x: xi, y: yi }), None],
                }))
            } else { None }
        } else if !pressed && was_pressed {
            // UP
            Some(HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Up,
                primary_pointer_id: 0,
                pointers: [Some(PointerSample { id: 0, x: last_x, y: last_y }), None],
            }))
        } else {
            None
        };

        if let Some(e) = hle {
            INPUT_EVENTS_CH.send(e).await;
        }

        was_pressed = pressed;
        if pressed { last_x = xi; last_y = yi; }

        Timer::after(Duration::from_millis(10)).await;
    }
}


