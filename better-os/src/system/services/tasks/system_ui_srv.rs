use defmt::{debug, info};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};

use crate::system::input::types::{HighLevelEvent, KeyAction, KeyCode, KeyEvent, MotionEvent};
use crate::system::ui::compositor::{animation::TransitionDirection, UICompositor};
use crate::system::ui::display_metrics;
use crate::system::ui::input::bus::SystemUiInputBus;
use crate::system::ui::input::gestures::{
    EdgeSwipeRecognizer, PointerEvent, PointerGesture, SwipeGestureUpdate,
};
use crate::system::ui::windowing::WindowManager;

#[embassy_executor::task]
pub async fn system_ui_gesture_task(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    let mut last_metrics = loop {
        if let Some(metrics) = display_metrics::metrics() {
            break metrics;
        }
        Timer::after(Duration::from_millis(10)).await;
    };
    let mut recognizer = EdgeSwipeRecognizer::new(
        last_metrics.logical_width as i32,
        last_metrics.logical_height as i32,
    );

    loop {
        let metrics = match display_metrics::metrics() {
            Some(m) => m,
            None => {
                Timer::after(Duration::from_millis(10)).await;
                continue;
            }
        };
        if metrics.logical_width != last_metrics.logical_width
            || metrics.logical_height != last_metrics.logical_height
        {
            recognizer
                .calibrate_frame_size(metrics.logical_width as i32, metrics.logical_height as i32);
            last_metrics = metrics;
        }

        let event = SystemUiInputBus::events().receive().await;
        let event_start = Instant::now();
        let mut consumed = false;
        let mut swipe_update = None;

        match event {
            HighLevelEvent::Motion(MotionEvent {
                action, pointers, ..
            }) => {
                if let Some(pointer) = pointers[0] {
                    let result = recognizer.observe(PointerEvent::new(pointer, action));
                    consumed = result.consumed;
                    swipe_update = result.update;
                }
            }
            HighLevelEvent::Key(key) => {
                consumed = handle_key_event(key, compositor, window_manager).await;
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

async fn handle_key_event(
    key: KeyEvent,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) -> bool {
    match (key.code, key.action) {
        (KeyCode::NextApp, KeyAction::Down | KeyAction::Repeat) => {
            debug!("Next-app key: action={:?}", key.action);
            let mut comp = compositor.lock().await;
            let mut wm = window_manager.lock().await;
            comp.animate_to_next_window(&mut wm).await;
            true
        }
        _ => false,
    }
}
