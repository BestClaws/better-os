use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::AsyncDisplay;
use crate::system::services::system_ui_srv::update_display_metrics;
use crate::system::ui::compositor::{
    animation::{ease_in_out_circular, ease_in_out_cubic, ease_out_bounce, AnimationConfig},
    core::UICompositor,
};
use crate::system::ui::display::Display;
use crate::system::ui::window_manager::WindowManager;

/// Target cadence for the compositor loop (~60 FPS).
const MIN_FRAME_TIME_MS: u64 = 16;

/// Compositor service task - handles UI rendering, animations, and display upkeep.
#[embassy_executor::task]
pub async fn ui_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
) {
    info!("Starting compositor service");

    // Initialize display hardware.
    {
        let mut display_lock = display.lock().await;
        display_lock.init().await;
        display_lock.set_brightness(0xFF).await;
        info!("Display initialized: brightness=100%");
    }

    // Attach display, configure compositor defaults, and publish dimensions to the UI layer.
    {
        let mut compositor_mut = compositor.lock().await;
        let display_facade: Display = Display::init(display).await;
        let width = display_facade.width();
        let height = display_facade.height();
        let negotiated_pixel_format = display_facade.pixel_format();
        compositor_mut.attach_display_service(display_facade);
        update_display_metrics(width, height);

        {
            let mut wm_mut = window_manager.lock().await;
            wm_mut.set_default_pixel_format(negotiated_pixel_format);
        }

        let animation_config = AnimationConfig {
            steps: 5,
            frame_delay_ms: 16,
            easing_fn: ease_in_out_cubic,
        };
        compositor_mut.set_animation_config(animation_config);

        info!("Compositor initialized with smooth animations");
    }

    let mut frame_count = 0u32;
    let mut last_perf_log = Instant::now();

    loop {
        let frame_start = Instant::now();

        // Idle refresh for dynamic content updates.
        Timer::after(Duration::from_millis(0)).await;

        let redraw_duration = {
            let redraw_start = Instant::now();
            let mut compositor_mut = compositor.lock().await;
            if let Some(focused_handle) = compositor_mut.focused_window_handle() {
                compositor_mut.request_redraw(focused_handle);
            }

            {
                let mut wm_mut = window_manager.lock().await;
                compositor_mut.process_redraws(&mut wm_mut).await;
            }

            redraw_start.elapsed()
        };

        frame_count += 1;
        if redraw_duration.as_millis() > 50
            || (frame_count % 300 == 0 && last_perf_log.elapsed().as_secs() >= 5)
        {
            info!(
                "Compositor frame: redraw={}ms, total={}ms",
                redraw_duration.as_millis(),
                frame_start.elapsed().as_millis()
            );
            last_perf_log = Instant::now();
        }

        maintain_frame_timing(frame_start).await;
    }
}

/// Maintain consistent frame timing to prevent system overload.
async fn maintain_frame_timing(loop_start: Instant) {
    let loop_duration = loop_start.elapsed();
    let min_frame_time = Duration::from_millis(MIN_FRAME_TIME_MS);

    if loop_duration < min_frame_time {
        Timer::after(min_frame_time - loop_duration).await;
    } else if loop_duration > Duration::from_millis(16) {
        debug!("Slow frame: {} ms", loop_duration.as_millis());
    }
}

/// Alternative animation configurations for different use cases.
#[allow(dead_code)]
mod animation_presets {
    use super::*;

    /// Fast, snappy animations for responsive feel.
    pub const FAST_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 6,
        frame_delay_ms: 15,
        easing_fn: ease_in_out_cubic,
    };

    /// Smooth, cinematic animations for premium feel.
    pub const CINEMATIC_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 15,
        frame_delay_ms: 25,
        easing_fn: ease_in_out_circular,
    };

    /// Playful bouncy animations.
    pub const BOUNCY_ANIMATIONS: AnimationConfig = AnimationConfig {
        steps: 20,
        frame_delay_ms: 20,
        easing_fn: ease_out_bounce,
    };
}

/// Debug utilities for performance monitoring.
#[cfg(feature = "debug-performance")]
mod debug_utils {
    use super::*;
    use heapless::Vec;

    /// Track frame timing statistics.
    pub struct PerformanceMonitor {
        frame_times: Vec<u32, 60>,
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

            if self.frame_count % 60 == 0 {
                let avg_time = self.average_frame_time();
                let max_time = self.max_frame_time();
                debug!(
                    "Performance: avg={}μs, max={}μs, fps≈{}",
                    avg_time,
                    max_time,
                    1_000_000 / avg_time
                );
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
