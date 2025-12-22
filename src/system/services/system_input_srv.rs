use alloc::boxed::Box;
use defmt::{debug, warn};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use embassy_time::{Duration, Timer, WithTimeout};

use crate::system::hal::button::{AsyncButton, ButtonState};
use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
use crate::system::hal::touch::AsyncTouch;
use crate::system::input::types::{
    HighLevelEvent, KeyAction, KeyCode, KeyEvent, MotionEvent, PointerSample, TouchAction,
};
use crate::system::kernel::config::resources::{
    FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR,
};
use crate::system::services::system_ui_srv::{SUI_ACK_CH, SUI_EVENT_CH};
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;

/// Raw input producers → dispatcher (normalized events)
pub static INPUT_EVENTS_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();

const ENCODER_COOLDOWN_MS: u64 = 200;
const TOUCH_POLL_MS: u64 = 10;
const TOUCH_COALESCE_TAXICAB_THRESHOLD: i32 = 1;
const SUI_ACK_TIMEOUT_MS: u64 = 250;

#[embassy_executor::task]
pub async fn button_reader_task(
    button: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>,
) {
    loop {
        let state = {
            let mut button = button.lock().await;
            button.next().await
        };

        let event = match state {
            ButtonState::Down => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Down,
            }),
            ButtonState::Up => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Up,
            }),
            ButtonState::Repeat => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Repeat,
            }),
        };

        INPUT_EVENTS_CH.send(event).await;
    }
}

#[embassy_executor::task]
pub async fn encoder_reader_task(
    encoder: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>,
) {
    loop {
        let Ok(state) = ({
            let mut encoder = encoder.lock().await;
            encoder.next().await
        }) else {
            warn!("Failed to read encoder state");
            continue;
        };

        // Map rotations to the primary confirm key to keep UX consistent.
        let event = match state {
            EncoderState::Ccw | EncoderState::Cw => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Down,
            }),
        };
        INPUT_EVENTS_CH.send(event).await;
        Timer::after(Duration::from_millis(ENCODER_COOLDOWN_MS)).await;
    }
}

#[embassy_executor::task]
pub async fn touch_reader_task(
    touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>,
) {
    use embassy_time::Instant;

    let mut was_pressed = false;
    let mut last_x = 0;
    let mut last_y = 0;
    let mut last_log = Instant::now();

    loop {
        let read_start = Instant::now();
        let (x_raw, y_raw, pressure) = {
            let mut touch = touch.lock().await;
            touch.read_xyz().await
        };
        let read_duration = read_start.elapsed();

        let pressed = pressure != 0;

        let mut x = (x_raw as u32 / FRAME_SCALE_FACTOR) as i32;
        let mut y = (y_raw as u32 / FRAME_SCALE_FACTOR) as i32;

        x = x.clamp(0, FRAME_BUFFER_WIDTH as i32 - 1);
        y = y.clamp(0, FRAME_BUFFER_HEIGHT as i32 - 1);

        let event = if pressed && !was_pressed {
            Some(HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Down,
                primary_pointer_id: 0,
                pointers: [Some(PointerSample { id: 0, x, y }), None],
            }))
        } else if pressed && was_pressed {
            let dx = (x - last_x).abs();
            let dy = (y - last_y).abs();
            if dx + dy >= TOUCH_COALESCE_TAXICAB_THRESHOLD {
                Some(HighLevelEvent::Motion(MotionEvent {
                    action: TouchAction::Move,
                    primary_pointer_id: 0,
                    pointers: [Some(PointerSample { id: 0, x, y }), None],
                }))
            } else {
                None
            }
        } else if !pressed && was_pressed {
            Some(HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Up,
                primary_pointer_id: 0,
                pointers: [
                    Some(PointerSample {
                        id: 0,
                        x: last_x,
                        y: last_y,
                    }),
                    None,
                ],
            }))
        } else {
            None
        };

        if let Some(event) = event {
            INPUT_EVENTS_CH.send(event).await;
        }

        if read_duration.as_millis() > 20 || (pressed && last_log.elapsed().as_secs() >= 2) {
            debug!(
                "Touch read: {}ms, pressed={}, pos=({},{})",
                read_duration.as_millis(),
                pressed,
                x,
                y
            );
            last_log = Instant::now();
        }

        was_pressed = pressed;
        if pressed {
            last_x = x;
            last_y = y;
        }

        Timer::after(Duration::from_millis(TOUCH_POLL_MS)).await;
    }
}

#[embassy_executor::task]
pub async fn input_dispatcher_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    use embassy_time::Instant;

    loop {
        let dispatch_start = Instant::now();
        let event = INPUT_EVENTS_CH.receive().await;

        SUI_EVENT_CH.send(event).await;

        let ack_start = Instant::now();
        let consumed = match SUI_ACK_CH
            .receive()
            .with_timeout(Duration::from_millis(SUI_ACK_TIMEOUT_MS))
            .await
        {
            Ok(consumed) => consumed,
            Err(_) => {
                warn!("SUI ACK timeout after {}ms", SUI_ACK_TIMEOUT_MS);
                false
            }
        };
        let ack_duration = ack_start.elapsed();

        if consumed {
            debug!("System UI consumed event");
            continue;
        }

        let mut compositor = compositor.lock().await;
        let mut window_manager = window_manager.lock().await;
        if let Some(handle) = compositor.focused_window_handle() {
            let _ = window_manager.try_send_input(handle, event).await;
            compositor.request_redraw(handle);
        } else {
            warn!("No focused window to receive input");
        }
        let total_duration = dispatch_start.elapsed();

        if total_duration.as_millis() > 50 || ack_duration.as_millis() > 30 {
            warn!(
                "Input dispatch slow: total={}ms, ack={}ms, consumed={}",
                total_duration.as_millis(),
                ack_duration.as_millis(),
                consumed
            );
        }
    }
}
