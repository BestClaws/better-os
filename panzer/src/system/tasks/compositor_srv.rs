use alloc::boxed::Box;
use alloc::string::ToString;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use crate::system::services::display::{Display, DisplayService};
use crate::system::window_manager::{WindowManager, WindowGeometry};
use crate::system::app_shell::AppShell;
use crate::system::compositor::{Compositor, TransitionType, Easing};
use crate::system::demo_apps::{ShapesDemo, GradientDemo, InfoDemo};
use crate::system::surface::DisplayInfo;
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;

/// Window Manager Compositor Service
/// Manages windows, apps, and compositing with animations
#[embassy_executor::task]
pub async fn window_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
) {
    info!("Starting window compositor service");

    let display_facade: Display = DisplayService::new(display).initialize().await;

    let width_u16 = display_facade.width() as u16;
    let height_u16 = display_facade.height() as u16;
    let negotiated_pixel_format = display_facade.pixel_format();
    let buffer_size = display_facade.framebuffer_size(width_u16 as u32, height_u16 as u32);
    let resolution = display_facade.resolution();
    let display_info = DisplayInfo::new(resolution.dpi as f32);

    info!(
        "Display initialized: {}x{} {:?} DPI={} (buffer {} bytes)",
        width_u16, height_u16, negotiated_pixel_format, 
        display_info.dpi as u32, buffer_size
    );

    // Allocate main display buffer
    let mut buffer = alloc::vec![0u8; buffer_size].into_boxed_slice();

    // Determine window format based on display format and available memory
    // LUMA4 (0): ~26KB per fullscreen window, grayscale
    // RGB565 (2): ~103KB per fullscreen window, full color
    let window_format = match negotiated_pixel_format {
        PixelFormat::Gray4 => 0,  // Use LUMA4 for Gray4 displays
        PixelFormat::Rgb565 => 2, // Use RGB565 for color displays
    };

    // Initialize window manager, app shell, and compositor
    let mut window_manager = WindowManager::new();
    let mut app_shell = AppShell::new(window_format, display_info);
    let mut compositor = Compositor::new();
    compositor.set_background_color(Color::rgba(20, 20, 30, 255));

    // Spawn demo applications with different window geometries
    let _app1_id = app_shell.spawn_app(
        "Shapes Demo".to_string(),
        Box::new(ShapesDemo::new("Shapes".to_string())),
        &mut window_manager,
        WindowGeometry {
            x: 0,
            y: 0,
            width: width_u16,
            height: height_u16,
        },
    );

    let _app2_id = app_shell.spawn_app(
        "Gradient Demo".to_string(),
        Box::new(GradientDemo::new("Gradient".to_string())),
        &mut window_manager,
        WindowGeometry {
            x: 0,
            y: 0,
            width: width_u16,
            height: height_u16,
        },
    );

    let _app3_id = app_shell.spawn_app(
        "Info Demo".to_string(),
        Box::new(InfoDemo::new("Info".to_string())),
        &mut window_manager,
        WindowGeometry {
            x: 0,
            y: 0,
            width: width_u16,
            height: height_u16,
        },
    );

    info!("Spawned {} demo apps", app_shell.app_count());

    // Get window IDs for switching
    let windows: alloc::vec::Vec<_> = app_shell
        .apps()
        .iter()
        .filter_map(|app| app.info.window_id)
        .collect();

    if !windows.is_empty() {
        compositor.switch_to_window_instant(windows[0]);
    }

    let mut frame_counter: u32 = 0;
    let mut last_frame_time = Instant::now();
    let mut current_window_idx = 0;
    let mut time_since_switch = Instant::now();
    const SWITCH_INTERVAL_MS: u64 = 3000; // Switch windows every 3 seconds

    loop {
        let frame_start = Instant::now();
        let time_delta = (frame_start - last_frame_time).as_millis() as f32 / 1000.0;
        last_frame_time = frame_start;

        // Switch windows periodically with animation
        if time_since_switch.elapsed().as_millis() > SWITCH_INTERVAL_MS && windows.len() > 1 {
            current_window_idx = (current_window_idx + 1) % windows.len();
            let next_window = windows[current_window_idx];

            // Cycle through different transition types
            let transition = match current_window_idx % 5 {
                0 => TransitionType::Fade,
                1 => TransitionType::SlideLeft,
                2 => TransitionType::SlideRight,
                3 => TransitionType::SlideTop,
                _ => TransitionType::Scale,
            };

            compositor.switch_to_window(next_window, transition, 500, Easing::EaseInOut);
            time_since_switch = Instant::now();
            
            info!("Switching to window {}", current_window_idx);
        }

        // Update compositor (handles transition progress)
        compositor.update();

        // Update all apps (they draw to their window surfaces)
        let delta_ms = (time_delta * 1000.0) as u32;
        app_shell.update_apps(&mut window_manager, delta_ms);

        // Composite windows to display buffer
        let render_start = Instant::now();
        match negotiated_pixel_format {
            PixelFormat::Gray4 => {
                let mut rasterizer = Luma4Rasterizer::new(&mut buffer, width_u16, height_u16);
                compositor.composite(&mut rasterizer, &window_manager);
            }
            PixelFormat::Rgb565 => {
                let mut rasterizer = Rgb565Rasterizer::new(&mut buffer, width_u16, height_u16);
                compositor.composite(&mut rasterizer, &window_manager);
            }
        }
        let render_time = render_start.elapsed().as_micros();

        // Draw to display
        let draw_start = Instant::now();
        display_facade.draw_full(&buffer).await;
        let flush_time = draw_start.elapsed().as_micros();

        let frame_time = frame_start.elapsed().as_micros();

        if frame_counter % 60 == 0 {
            info!(
                "frame={} render={}us flush={}us total={}us windows={}",
                frame_counter,
                render_time,
                flush_time,
                frame_time,
                window_manager.window_count()
            );
        }

        frame_counter = frame_counter.wrapping_add(1);
        Timer::after(Duration::from_millis(16)).await;
    }
}
