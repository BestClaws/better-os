use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::compositor::{animation::{AnimationConfig, TransitionDirection, ease_in_out_cubic, ease_in_out_circular, ease_out_bounce}, core::UICompositor};
use crate::system::services::input_srv::{SUI_EVENT_CH, SUI_ACK_CH};
// CriticalSectionRawMutex already imported above
use crate::system::ui::window_manager::WindowManager;
use crate::system::ui::display::Display;
// PixelFormat is determined via Display facade; no direct use here

/// Service loop timing constants
const MIN_FRAME_TIME_MS: u64 = 16; // ~60 FPS max

/// Compositor service task - handles UI rendering, input events, and animations
///
/// This service:
/// - Initializes display hardware
/// - Processes user input events and routes them appropriately
/// - Manages window transitions and animations
/// - Provides idle refresh for dynamic content (clocks, sensors, etc.)
/// - Maintains consistent frame timing for smooth operation
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
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
        let mut compositor_mut = compositor.lock().await;
        let display_facade: Display = Display::init(display).await;
        let negotiated_pixel_format = display_facade.pixel_format();
        compositor_mut.attach_display_service(display_facade);
        // Align WindowManager default format with negotiated Display format
        {
            let mut wm_mut = window_manager.lock().await;
            wm_mut.set_default_pixel_format(negotiated_pixel_format);
        }

        // Configure smooth animations with cubic easing
        let animation_config = AnimationConfig {
            steps: 5,
            frame_delay_ms: 16,
            easing_fn: ease_in_out_cubic,
        };
        compositor_mut.set_animation_config(animation_config);

        info!("Compositor initialized with smooth animations");
    }

    // System UI gesture consumption is integrated below.

    // Main service loop
    let mut edge_swipe = EdgeSwipeRecognizer::new();
    loop {
        let frame_start = Instant::now();

        // Process System UI events with a short idle to keep frames flowing
        let handled_sui_events = handle_system_ui_events(compositor, window_manager, &mut edge_swipe).await;
        if !handled_sui_events {
            // Idle refresh for dynamic content updates
            Timer::after(Duration::from_millis(0)).await;
            let mut compositor_mut = compositor.lock().await;
            if let Some(focused_handle) = compositor_mut.focused_window_handle() {
                compositor_mut.request_redraw(focused_handle);
            }
            let mut wm_mut = window_manager.lock().await;
            compositor_mut.process_redraws(&mut wm_mut).await;
        }

        // Maintain consistent frame timing
        maintain_frame_timing(frame_start).await;
    }
}

/// Request redraw for currently focused window
async fn request_redraw_focused_window(compositor: &mut UICompositor) {
    if let Some(focused_handle) = compositor.focused_window_handle() {
        compositor.request_redraw(focused_handle);
    }
}

/// Recognizes edge swipe gestures for System UI window transitions.
struct EdgeSwipeRecognizer {
    tracking: bool,
    start_x: i32,
    start_y: i32,
    from_left: bool,
    from_right: bool,
    fired: bool,
}

impl EdgeSwipeRecognizer {
    const EDGE_THRESHOLD: i32 = 24;
    const MIN_SWIPE_DISTANCE: i32 = 60;
    const MIN_SWIPE_ANGLE_TOLERANCE: i32 = 20;

    fn new() -> Self {
        Self { tracking: false, start_x: 0, start_y: 0, from_left: false, from_right: false, fired: false }
    }

    fn reset(&mut self) {
        self.tracking = false;
        self.fired = false;
    }

    /// Process a motion sample; returns (consumed, optional transition)
    fn process_motion(&mut self, x: i32, y: i32, action: crate::system::input::types::TouchAction, frame_w: i32) -> (bool, Option<TransitionDirection>) {
        use crate::system::input::types::TouchAction;
        match action {
            TouchAction::Down => {
                self.reset();
                self.from_left = x <= Self::EDGE_THRESHOLD;
                self.from_right = x >= (frame_w - Self::EDGE_THRESHOLD);
                if self.from_left || self.from_right {
                    self.tracking = true;
                    self.start_x = x;
                    self.start_y = y;
                }
                (false, None)
            }
            TouchAction::Move => {
                if self.tracking && !self.fired {
                    let dx = x - self.start_x;
                    let dy = (y - self.start_y).abs();
                    if dy <= Self::MIN_SWIPE_ANGLE_TOLERANCE {
                        if self.from_left && dx > Self::MIN_SWIPE_DISTANCE {
                            self.fired = true;
                            return (true, Some(TransitionDirection::Next));
                        } else if self.from_right && (-dx) > Self::MIN_SWIPE_DISTANCE {
                            self.fired = true;
                            return (true, Some(TransitionDirection::Previous));
                        }
                    }
                }
                (false, None)
            }
            TouchAction::Up => {
                self.reset();
                (false, None)
            }
        }
    }
}

/// Consume System UI events (gestures) and drive window transitions.
/// Returns true if any SUI event was processed.
/// Process System UI input stream and drive transitions. Returns true if any SUI event was handled.
async fn handle_system_ui_events(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    recognizer: &mut EdgeSwipeRecognizer,
) -> bool {
    use crate::system::input::types::{HighLevelEvent, MotionEvent};

    let mut processed_any = false;

    // Try non-blocking receive on SUI_EVENT_CH to keep frame cadence
    while let Ok(ev) = SUI_EVENT_CH.try_receive() {
        let mut consumed = false;
        if let HighLevelEvent::Motion(MotionEvent { action, pointers, .. }) = ev {
            if let Some(p) = pointers[0] {
                let frame_w = {
                    let comp = compositor.lock().await; // brief lock to read dimensions
                    comp.display_dimensions().map(|(w, _)| w as i32).unwrap_or(0)
                };
                let (was_consumed, transition) = recognizer.process_motion(p.x, p.y, action, frame_w);
                consumed = was_consumed;

                if let Some(dir) = transition {
                    processed_any = true;
                    match dir {
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
            }
        }

        SUI_ACK_CH.send(consumed).await;
    }

    processed_any
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
        debug!("Slow frame: {} ms", loop_duration.as_millis());
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