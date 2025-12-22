use core::sync::atomic::{AtomicI32, Ordering};

use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::system::input::types::{HighLevelEvent, MotionEvent};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};
use crate::system::ui::compositor::{animation::TransitionDirection, core::UICompositor};
use crate::system::ui::gestures::edge_swipe::EdgeSwipeRecognizer;
use crate::system::ui::window_manager::WindowManager;

/// Dispatcher → System UI consumer (events for gesture/UI handling)
pub static SUI_EVENT_CH: Channel<CriticalSectionRawMutex, HighLevelEvent, 64> = Channel::new();
/// System UI → dispatcher (per-event consumed acknowledgement)
pub static SUI_ACK_CH: Channel<CriticalSectionRawMutex, bool, 64> = Channel::new();

static FRAME_WIDTH_HINT: AtomicI32 = AtomicI32::new(FRAME_BUFFER_WIDTH as i32);
static FRAME_HEIGHT_HINT: AtomicI32 = AtomicI32::new(FRAME_BUFFER_HEIGHT as i32);

/// Update the logical framebuffer dimensions used for gesture heuristics.
pub fn update_display_metrics(width: u32, height: u32) {
    FRAME_WIDTH_HINT.store(width as i32, Ordering::Relaxed);
    FRAME_HEIGHT_HINT.store(height as i32, Ordering::Relaxed);
}

fn current_dimensions() -> (i32, i32) {
    (
        FRAME_WIDTH_HINT.load(Ordering::Relaxed),
        FRAME_HEIGHT_HINT.load(Ordering::Relaxed),
    )
}

#[embassy_executor::task]
pub async fn system_ui_gesture_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    let mut last_dims = current_dimensions();
    let mut recognizer = EdgeSwipeRecognizer::new(last_dims.0, last_dims.1);

    loop {
        let dims = current_dimensions();
        if dims != last_dims {
            recognizer.calibrate_frame_size(dims.0, dims.1);
            last_dims = dims;
        }

        let event = SUI_EVENT_CH.receive().await;
        let event_start = Instant::now();
        let mut consumed = false;
        let mut transition = None;

        if let HighLevelEvent::Motion(MotionEvent {
            action, pointers, ..
        }) = event
        {
            if let Some(pointer) = pointers[0] {
                let (was_consumed, maybe_transition) =
                    recognizer.process_sample(pointer.x, pointer.y, action);
                consumed = was_consumed;
                transition = maybe_transition;
            }
        }

        SUI_ACK_CH.send(consumed).await;

        if let Some(direction) = transition {
            execute_transition(direction, compositor, window_manager).await;
        }

        let event_duration = event_start.elapsed();
        if event_duration.as_millis() > 20 {
            info!(
                "SUI event slow: {}ms, consumed={}",
                event_duration.as_millis(),
                consumed
            );
        }
    }
}

async fn execute_transition(
    direction: TransitionDirection,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    match direction {
        TransitionDirection::Next => {
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.animate_to_next_window(&mut wm).await;
            request_redraw_focused_window(&mut comp).await;
            comp.process_redraws(&mut wm).await;
        }
        TransitionDirection::Previous => {
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.animate_to_previous_window(&mut wm).await;
            request_redraw_focused_window(&mut comp).await;
            comp.process_redraws(&mut wm).await;
        }
    }
}

async fn request_redraw_focused_window(compositor: &mut UICompositor) {
    if let Some(focused_handle) = compositor.focused_window_handle() {
        compositor.request_redraw(focused_handle);
    }
}
