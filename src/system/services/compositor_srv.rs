use alloc::boxed::Box;
use defmt::{debug, info, warn};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::compositor::system_ui_consume_events;
use crate::system::ui::compositor::{AnimationConfig, TransitionDirection, UICompositor, SUI_COMMAND_CH};
use crate::system::ui::window_manager::WindowManager;
use crate::system::ui::compositor::{ease_in_out_cubic, ease_in_out_circular, ease_out_bounce};
use crate::system::ui::display::Display;

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
        let mut compositor_lock = compositor.lock().await;
        static mut DISPLAY: Option<Display> = None;
        let d = unsafe { DISPLAY.get_or_insert(Display::init(display).await) };
        compositor_lock.attach_display_service(unsafe { DISPLAY.as_ref().unwrap() });

        // Configure smooth animations with cubic easing
        let animation_config = AnimationConfig {
            steps: 5,
            frame_delay_ms: 16,
            easing_fn: ease_in_out_cubic,
        };
        compositor_lock.set_animation_config(animation_config);

        info!("Compositor initialized with smooth animations");
    }

    // SUI consumer is spawned from start.rs

    // Main service loop
    loop {
        let loop_start = Instant::now();

        // React to SUI swipe decisions if any
        if let Ok(dir) = SUI_COMMAND_CH.try_receive() {
            match dir {
                TransitionDirection::Next => {
                    let mut compositor_lock = compositor.lock().await;
                    let mut wm_lock = window_manager.lock().await;
                    compositor_lock.animate_to_next_window(&mut wm_lock).await;
                    request_focused_window_redraw(&mut compositor_lock).await;
                    compositor_lock.process_redraws(&mut wm_lock).await;
                }
                TransitionDirection::Previous => {
                    let mut compositor_lock = compositor.lock().await;
                    let mut wm_lock = window_manager.lock().await;
                    compositor_lock.animate_to_previous_window(&mut wm_lock).await;
                    request_focused_window_redraw(&mut compositor_lock).await;
                    compositor_lock.process_redraws(&mut wm_lock).await;
                }
            }
        } else {
            // Idle refresh for dynamic content updates
            Timer::after(Duration::from_millis(0)).await;
            let mut compositor_lock = compositor.lock().await;
            if let Some(focused_handle) = compositor_lock.focused_window_handle() {
                compositor_lock.request_redraw(focused_handle);
            }
            let mut wm_lock = window_manager.lock().await;
            compositor_lock.process_redraws(&mut wm_lock).await;
        }

        // Maintain consistent frame timing
        maintain_frame_timing(loop_start).await;
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