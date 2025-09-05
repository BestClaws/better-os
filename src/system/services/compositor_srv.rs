use alloc::boxed::Box;
use defmt::{debug, info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};
use crate::system::ui::compositor::{AnimationConfig, TransitionDirection, UICompositor};
use crate::system::ui::compositor::{ease_in_out_cubic, ease_in_out_circular, ease_out_bounce};

/// Service loop timing constants
const MIN_FRAME_TIME_MS: u64 = 16; // ~60 FPS max
const INPUT_POLL_TIMEOUT_MS: u64 = 1;

/// Compositor service task - handles UI rendering, input events, and animations
///
/// This service:
/// - Initializes display hardware
/// - Processes user input events and routes them appropriately
/// - Manages window transitions and animations
/// - Provides idle refresh for dynamic content (clocks, sensors, etc.)
/// - Maintains consistent frame timing for smooth operation
#[embassy_executor::task]
pub async fn compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
) {
    info!("Starting compositor service");

    // Initialize display hardware
    {
        let mut display_lock = display.lock().await;
        display_lock.init().await;
        display_lock.set_brightness(0xFF).await; // Maximum brightness
        info!("Display initialized: brightness=100%");
    }

    // Attach display to compositor and configure animations
    {
        let mut compositor_lock = compositor.lock().await;
        compositor_lock.attach_display(display);

        // Configure smooth animations with cubic easing
        let animation_config = AnimationConfig {
            steps: 5,
            frame_delay_ms: 16,
            easing_fn: ease_in_out_cubic,
        };
        compositor_lock.set_animation_config(animation_config);

        info!("Compositor initialized with smooth animations");
    }

    // Main service loop
    loop {
        let loop_start = Instant::now();

        // Process input events
        let input_action = process_input_events().await;

        // Execute requested actions
        match input_action {
            InputAction::AnimateNext => {
                debug!("Processing next window animation");
                let mut compositor_lock = compositor.lock().await;
                compositor_lock.animate_to_next_window().await;
                request_focused_window_redraw(&mut compositor_lock).await;
                compositor_lock.process_redraws().await;
            }

            InputAction::AnimatePrevious => {
                debug!("Processing previous window animation");
                let mut compositor_lock = compositor.lock().await;
                compositor_lock.animate_to_previous_window().await;
                request_focused_window_redraw(&mut compositor_lock).await;
                compositor_lock.process_redraws().await;
            }

            InputAction::ToggleView => {
                debug!("Processing view mode toggle");
                let mut compositor_lock = compositor.lock().await;
                compositor_lock.toggle_display_mode().await;
                request_focused_window_redraw(&mut compositor_lock).await;
                compositor_lock.process_redraws().await;
            }

            InputAction::ForwardToWindow(event) => {
                debug!("Forwarding input to focused window: {:?}", event);
                forward_input_to_focused_window(compositor, event).await;

                // Trigger redraw after input processing
                let mut compositor_lock = compositor.lock().await;
                request_focused_window_redraw(&mut compositor_lock).await;
                compositor_lock.process_redraws().await;
            }

            InputAction::IdleRefresh => {
                // Idle refresh for dynamic content updates
                Timer::after(Duration::from_millis(0)).await;
                let mut compositor_lock = compositor.lock().await;
                if let Some(focused_handle) = compositor_lock.focused_window_handle() {
                    compositor_lock.request_redraw(focused_handle);
                    compositor_lock.process_redraws().await;
                }
            }
        }

        // Maintain consistent frame timing
        maintain_frame_timing(loop_start).await;
    }
}

/// Categorizes input events into actionable items
#[derive(Debug)]
enum InputAction {
    /// Animate to next window
    AnimateNext,
    /// Animate to previous window
    AnimatePrevious,
    /// Toggle between single/split view
    ToggleView,
    /// Forward input event to focused window
    ForwardToWindow(HumanInputEvent),
    /// No input - perform idle refresh
    IdleRefresh,
}

/// Process all pending input events and determine action
async fn process_input_events() -> InputAction {
    // Drain all available input events, keeping only the most recent
    let mut latest_event = None;
    let mut event_count = 0;

    while let Ok(event) = HUMAN_INPUT_CH.try_receive() {
        latest_event = Some(event);
        event_count += 1;
    }

    if event_count > 1 {
        debug!("Processed {} input events, using latest", event_count);
    }

    match latest_event {
        Some(HumanInputEvent::OkPressed) => {
            // OK button triggers next window animation
            InputAction::AnimateNext
        }

        Some(HumanInputEvent::Touch(x, y)) => {
            // Touch gestures for window navigation
            process_touch_gesture(x, y)
        }

        // Some(HumanInputEvent::SwipeLeft) => {
        //     InputAction::AnimateNext
        // }
        //
        // Some(HumanInputEvent::SwipeRight) => {
        //     InputAction::AnimatePrevious
        // }
        //
        // Some(HumanInputEvent::LongPress) => {
        //     InputAction::ToggleView
        // }

        Some(other_event) => {
            // Forward all other events to the focused window
            InputAction::ForwardToWindow(other_event)
        }

        None => {
            // No input events - perform idle refresh
            InputAction::IdleRefresh
        }
    }
}

/// Process touch gestures for intuitive navigation
///
/// Touch zones:
/// - Top half: Previous/Next window based on left/right
/// - Bottom half: Forward to application
fn process_touch_gesture(x: i32, y: i32) -> InputAction {
    use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

    let screen_mid_y = FRAME_BUFFER_HEIGHT as i32 / 2;
    let screen_mid_x = FRAME_BUFFER_WIDTH as i32 / 2;

    if y < screen_mid_y {
        // Top half - window navigation
        if x < screen_mid_x {
            debug!("Touch gesture: Navigate previous (top-left)");
            InputAction::AnimatePrevious
        } else {
            debug!("Touch gesture: Navigate next (top-right)");
            InputAction::AnimateNext
        }
    } else {
        // Bottom half - forward to application
        debug!("Touch gesture: Forward to app (bottom)");
        InputAction::ForwardToWindow(HumanInputEvent::Touch(x, y))
    }
}

/// Forward input event to the currently focused window
async fn forward_input_to_focused_window(
    compositor: &Mutex<CriticalSectionRawMutex, UICompositor>,
    event: HumanInputEvent,
) {
    let mut compositor_lock = compositor.lock().await;

    if let Some(focused_handle) = compositor_lock.focused_window_handle() {
        if let Some(window) = compositor_lock.get_window_mut(focused_handle) {
            if let Some(sender) = window.input_sender().await {
                match sender.try_send(event) {
                    Ok(_) => {
                        debug!("Input forwarded to window {:?}", focused_handle);
                    }
                    Err(_) => {
                        warn!("Failed to forward input: window input queue full");
                    }
                }
            } else {
                warn!("No input sender available for focused window");
            }
        } else {
            warn!("Focused window not found for input forwarding");
        }
    } else {
        debug!("No focused window to forward input to");
    }
}

/// Request redraw for currently focused window
async fn request_focused_window_redraw(compositor: &mut UICompositor) {
    if let Some(focused_handle) = compositor.focused_window_handle() {
        compositor.request_redraw(focused_handle);
    }
}

/// Maintain consistent frame timing to prevent system overload
async fn maintain_frame_timing(loop_start: Instant) {
    let loop_duration = loop_start.elapsed();
    let min_frame_time = Duration::from_millis(MIN_FRAME_TIME_MS);

    if loop_duration < min_frame_time {
        let sleep_time = min_frame_time - loop_duration;
        Timer::after(sleep_time).await;
    } else if loop_duration > Duration::from_millis(16) {
        // Warn if frame took too long (may indicate performance issues)
        warn!("Slow frame: {} ms", loop_duration.as_millis());
    }
}

/// Alternative animation configurations for different use cases
#[allow(dead_code)]
mod animation_presets {
    use super::*;

    /// Fast, snappy animations for responsive feel
    pub const FAST_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 6,
        frame_delay_ms: 15,
        easing_fn: ease_in_out_cubic,
    };

    /// Smooth, cinematic animations for premium feel
    pub const CINEMATIC_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 15,
        frame_delay_ms: 25,
        easing_fn: ease_in_out_circular,
    };

    /// Playful bouncy animations
    pub const BOUNCY_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 20,
        frame_delay_ms: 20,
        easing_fn: ease_out_bounce,
    };
}

/// Debug utilities for performance monitoring
#[cfg(feature = "debug-performance")]
mod debug_utils {
    use super::*;
    use heapless::Vec;

    /// Track frame timing statistics
    pub struct PerformanceMonitor {
        frame_times: Vec<u32, 60>, // Last 60 frame times in microseconds
        frame_count: u32,
    }

    impl PerformanceMonitor {
        pub fn new() -> Self {
            Self {
                frame_times: Vec::new(),
                frame_count: 0,
            }
        }

        pub fn record_frame(&mut self, duration: Duration) {
            let micros = duration.as_micros() as u32;

            if self.frame_times.len() >= 60 {
                self.frame_times.remove(0);
            }

            self.frame_times.push(micros).ok();
            self.frame_count += 1;

            // Log performance stats every 60 frames
            if self.frame_count % 60 == 0 {
                let avg_time = self.average_frame_time();
                let max_time = self.max_frame_time();
                debug!("Performance: avg={}μs, max={}μs, fps≈{}",
                       avg_time, max_time, 1_000_000 / avg_time);
            }
        }

        fn average_frame_time(&self) -> u32 {
            if self.frame_times.is_empty() {
                return 0;
            }

            let sum: u32 = self.frame_times.iter().sum();
            sum / self.frame_times.len() as u32
        }

        fn max_frame_time(&self) -> u32 {
            self.frame_times.iter().max().copied().unwrap_or(0)
        }
    }
}