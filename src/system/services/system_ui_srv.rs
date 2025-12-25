use core::sync::atomic::{AtomicI32, Ordering};

use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Instant;

use crate::system::input::types::{HighLevelEvent, MotionEvent};
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};
use crate::system::ui::compositor::{animation::TransitionDirection, UICompositor};
use crate::system::ui::input::bus::SystemUiInputBus;
use crate::system::ui::input::gestures::edge_swipe::{EdgeSwipeRecognizer, SwipeGestureUpdate};
use crate::system::ui::windowing::WindowManager;

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

        let event = SystemUiInputBus::events().receive().await;
        let event_start = Instant::now();
        let mut consumed = false;
        let mut swipe_update = None;

        if let HighLevelEvent::Motion(MotionEvent {
            action, pointers, ..
        }) = event
        {
            if let Some(pointer) = pointers[0] {
                let (was_consumed, maybe_update) =
                    recognizer.process_sample(pointer.x, pointer.y, action);
                consumed = was_consumed;
                swipe_update = maybe_update;
            }
        }

        SystemUiInputBus::acknowledgements().send(consumed).await;

        if let Some(update) = swipe_update {
            handle_swipe_update(update, compositor, window_manager).await;
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

async fn handle_swipe_update(
    update: SwipeGestureUpdate,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    match update {
        SwipeGestureUpdate::Preview {
            direction,
            progress,
        } => {
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.preview_transition(&mut wm, direction, progress).await;
        }
        SwipeGestureUpdate::Commit {
            direction,
            progress,
        } => {
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.commit_transition(&mut wm, direction, progress).await;
            request_redraw_focused_window(&mut comp).await;
            comp.process_redraws(&mut wm).await;
        }
        SwipeGestureUpdate::Cancel {
            direction,
            progress,
        } => {
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.cancel_transition(&mut wm, direction, progress).await;
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
