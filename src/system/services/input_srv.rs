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
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;

/// Consolidated input service.
///
/// Responsibilities:
/// - Read raw inputs (button, encoder, touch) and publish debounced/high-level events
/// - Route events to System UI first; if rejected, forward to focused app
///
/// Channels:
/// - INPUT_EVENTS_CH: Raw input readers → dispatcher (high-level events)
/// - SUI_EVENT_CH: Dispatcher → System UI consumer (events for gesture/UI handling)
/// - SUI_ACK_CH: System UI → dispatcher (per-event consumed acknowledgement)
pub static INPUT_EVENTS_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();
pub static SUI_EVENT_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();
pub static SUI_ACK_CH: Channel<CriticalSectionRawMutex, bool, 64> = Channel::new();

// Timing and thresholds
const ENCODER_COOLDOWN_MS: u64 = 200;
const TOUCH_POLL_MS: u64 = 10;
const TOUCH_COALESCE_TAXICAB_THRESHOLD: i32 = 1; // dx+dy >= 1 pixel
const DISPATCH_ACK_TIMEOUT_MS: u64 = 250;

#[embassy_executor::task]
pub async fn button_reader_task(
    button: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>,
) {
    loop {
        let state = {
            let mut b = button.lock().await;
            b.next().await
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
            let mut e = encoder.lock().await;
            e.next().await
        }) else {
            warn!("Failed to read encoder state");
            continue;
        };

        let event = match state {
            // Map both directions to OK Down to keep single-button semantics minimal
            EncoderState::Ccw => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Down,
            }),
            EncoderState::Cw => HighLevelEvent::Key(KeyEvent {
                code: KeyCode::Ok,
                action: KeyAction::Down,
            }),
        };
        INPUT_EVENTS_CH.send(event).await;
        Timer::after(Duration::from_millis(ENCODER_COOLDOWN_MS)).await; // cooldown
    }
}

/// Converts raw xyz polling into MotionEvent DOWN/MOVE/UP with bounds/clamping and coalescing.
#[embassy_executor::task]
pub async fn touch_reader_task(
    touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>,
) {
    use embassy_time::Instant;
    let mut was_pressed = false;
    let mut last_x: i32 = 0;
    let mut last_y: i32 = 0;
    let mut last_log_time = Instant::now();

    loop {
        let read_start = Instant::now();
        let (x, y, z) = {
            let mut t = touch.lock().await;
            t.read_xyz().await
        };
        let read_duration = read_start.elapsed();

        let pressed = z != 0;

        let mut xi = (x as u32 / FRAME_SCALE_FACTOR) as i32;
        let mut yi = (y as u32 / FRAME_SCALE_FACTOR) as i32;

        if xi < 0 {
            xi = 0;
        }
        if yi < 0 {
            yi = 0;
        }
        if xi >= FRAME_BUFFER_WIDTH as i32 {
            xi = FRAME_BUFFER_WIDTH as i32 - 1;
        }
        if yi >= FRAME_BUFFER_HEIGHT as i32 {
            yi = FRAME_BUFFER_HEIGHT as i32 - 1;
        }

        let hle = if pressed && !was_pressed {
            Some(HighLevelEvent::Motion(MotionEvent {
                action: TouchAction::Down,
                primary_pointer_id: 0,
                pointers: [
                    Some(PointerSample {
                        id: 0,
                        x: xi,
                        y: yi,
                    }),
                    None,
                ],
            }))
        } else if pressed && was_pressed {
            let dx = (xi - last_x).abs();
            let dy = (yi - last_y).abs();
            if dx + dy >= TOUCH_COALESCE_TAXICAB_THRESHOLD {
                Some(HighLevelEvent::Motion(MotionEvent {
                    action: TouchAction::Move,
                    primary_pointer_id: 0,
                    pointers: [
                        Some(PointerSample {
                            id: 0,
                            x: xi,
                            y: yi,
                        }),
                        None,
                    ],
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

        if let Some(e) = hle {
            INPUT_EVENTS_CH.send(e).await;
        }

        // Log touch performance issues (every 2 seconds max)
        if read_duration.as_millis() > 20 || (pressed && last_log_time.elapsed().as_secs() >= 2) {
            defmt::debug!(
                "Touch read: {}ms, pressed={}, pos=({},{})",
                read_duration.as_millis(),
                pressed,
                xi,
                yi
            );
            last_log_time = Instant::now();
        }

        was_pressed = pressed;
        if pressed {
            last_x = xi;
            last_y = yi;
        }

        Timer::after(Duration::from_millis(TOUCH_POLL_MS)).await;
    }
}

/// Routes events to System UI first; if unconsumed, forwards to focused window.
#[embassy_executor::task]
pub async fn input_dispatcher_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    use embassy_time::Instant;

    loop {
        let dispatch_start = Instant::now();
        let event = INPUT_EVENTS_CH.receive().await;

        // Send to System UI
        SUI_EVENT_CH.send(event).await;

        // Wait for ACK with timeout
        let ack_start = Instant::now();
        let consumed = match SUI_ACK_CH
            .receive()
            .with_timeout(Duration::from_millis(DISPATCH_ACK_TIMEOUT_MS))
            .await
        {
            Ok(val) => val,
            Err(_) => {
                defmt::info!("SUI ACK timeout after {}ms", DISPATCH_ACK_TIMEOUT_MS);
                false
            }
        };
        let ack_duration = ack_start.elapsed();

        if consumed {
            debug!("SUI consumed event");
            continue;
        }

        // Forward to focused app
        let app_forward_start = Instant::now();
        let mut comp = compositor.lock().await;
        let mut wm = window_manager.lock().await;
        if let Some(handle) = comp.focused_window_handle() {
            let _ = wm.try_send_input(handle, event).await;
            comp.request_redraw(handle);
        } else {
            warn!("No focused window to receive input");
        }
        let total_duration = dispatch_start.elapsed();

        // Log slow event processing
        if total_duration.as_millis() > 50 || ack_duration.as_millis() > 30 {
            defmt::info!(
                "Input dispatch slow: total={}ms, ack={}ms, consumed={}",
                total_duration.as_millis(),
                ack_duration.as_millis(),
                consumed
            );
        }
    }
}
