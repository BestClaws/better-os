use alloc::boxed::Box;
use alloc::string::ToString;
use defmt::{error, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::display::{AsyncDisplay, PixelFormat};
use crate::system::services::display::{Display, DisplayService};
use crate::system::window_manager::{WindowManager, WindowGeometry};
use crate::system::app_shell::{AppShell, AppId};
use crate::system::compositor::{Compositor, TransitionType, Easing};
use crate::system::demo_apps::{ShapesDemo, GradientDemo, WidgetDemo};
use crate::system::surface::DisplayInfo;
use crate::system::vendor::chipone::ft3x68::{Ft3x68, TouchEvent};
use crate::system::input::InputEvent;
use gfx::colors::Color;
use gfx::luma4::Luma4Rasterizer;
use gfx::rgb565::Rgb565Rasterizer;

/// Window Manager Compositor Service
/// Manages windows, apps, and compositing with animations
#[embassy_executor::task]
pub async fn window_compositor_service(
    display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    mut touch: Option<Ft3x68<esp_hal::i2c::master::I2c<'static, esp_hal::Async>>>,
) {
    info!("Starting window compositor service");

    // Initialize touch if available
    if let Some(ref mut t) = touch {
        if let Err(e) = t.init().await {
            error!("Failed to initialize touch: {:?}", e);
        }
    }

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

    // Spawn demo applications - WidgetDemo first for immediate testing
    let app1_id = app_shell.spawn_app(
        "Widget Demo".to_string(),
        Box::new(WidgetDemo::new("Widgets".to_string(), AppId::new(999))), // Dummy ID
        &mut window_manager,
        WindowGeometry {
            x: 0,
            y: 0,
            width: width_u16,
            height: height_u16,
        },
    );

    let app2_id = app_shell.spawn_app(
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

    let app3_id = app_shell.spawn_app(
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

    info!("Spawned {} demo apps", app_shell.app_count());

    // Get window IDs and app IDs for switching
    let windows: alloc::vec::Vec<_> = app_shell
        .apps()
        .iter()
        .filter_map(|app| app.info.window_id)
        .collect();
    
    let app_ids: alloc::vec::Vec<_> = app_shell
        .apps()
        .iter()
        .map(|app| app.info.id)
        .collect();

    if !windows.is_empty() {
        compositor.switch_to_window_instant(windows[0]);
        // Set initial focus to first app
        if !app_ids.is_empty() {
            app_shell.focus_app(app_ids[0]);
        }
    }

    let mut frame_counter: u32 = 0;
    let mut last_frame_time = Instant::now();
    let mut current_window_idx = 0;
    
    // Simple swipe detection - just track start and end
    let mut swipe_start_x: Option<i32> = None;
    let mut swipe_start_y: Option<i32> = None;
    let mut swipe_start_time: Option<Instant> = None;
    
    // Touch coordinate scaling: FT3x68 reports 0-410, display is 205x251
    const TOUCH_SCALE_X: f32 = 0.5; // 410 -> 205
    const TOUCH_SCALE_Y: f32 = 0.5; // ~502 -> 251

    loop {
        let frame_start = Instant::now();
        let time_delta = (frame_start - last_frame_time).as_millis() as f32 / 1000.0;
        last_frame_time = frame_start;

        // Handle touch input
        if let Some(ref mut touch_ctrl) = touch {
            if let Ok(Some(point)) = touch_ctrl.read_touch().await {
                let pressed = point.event != TouchEvent::LiftUp;
                
                // Scale touch coordinates
                let scaled_x = (point.x as f32 * TOUCH_SCALE_X) as i32;
                let scaled_y = (point.y as f32 * TOUCH_SCALE_Y) as i32;
                
                if pressed {
                    // Record start position
                    if swipe_start_x.is_none() {
                        swipe_start_x = Some(scaled_x);
                        swipe_start_y = Some(scaled_y);
                        swipe_start_time = Some(Instant::now());
                    }
                    
                    // Always forward touch to apps
                    app_shell.queue_input(InputEvent::Touch {
                        x: scaled_x as u16,
                        y: scaled_y as u16,
                        pressed: true,
                    });
                } else {
                    // Touch release - check for swipe
                    if let (Some(start_x), Some(start_y), Some(start_time)) = 
                        (swipe_start_x, swipe_start_y, swipe_start_time) {
                        
                        let dx = scaled_x - start_x;
                        let dy = scaled_y - start_y;
                        let elapsed_ms = start_time.elapsed().as_millis() as u64;
                        
                        // Simple swipe detection: horizontal movement > 80px, time < 600ms, more horizontal than vertical
                        let is_swipe = dx.abs() > 80 && 
                                      elapsed_ms < 600 && 
                                      dx.abs() > dy.abs() * 2;
                        
                        if is_swipe {
                            // Determine direction and switch window
                            let target_idx = if dx > 0 {
                                // Swipe right = previous window
                                if current_window_idx == 0 {
                                    windows.len() - 1
                                } else {
                                    current_window_idx - 1
                                }
                            } else {
                                // Swipe left = next window
                                (current_window_idx + 1) % windows.len()
                            };
                            
                            let transition_type = if dx > 0 {
                                TransitionType::SlideLeft
                            } else {
                                TransitionType::SlideRight
                            };
                            
                            compositor.switch_to_window(
                                windows[target_idx],
                                transition_type,
                                250,
                                Easing::EaseOut,
                            );
                            
                            current_window_idx = target_idx;
                            if current_window_idx < app_ids.len() {
                                app_shell.focus_app(app_ids[current_window_idx]);
                            }
                            
                            info!("Swipe {} -> window {}", if dx > 0 { "→" } else { "←" }, current_window_idx);
                        } else {
                            // Not a swipe - forward touch release to app
                            app_shell.queue_input(InputEvent::Touch {
                                x: scaled_x as u16,
                                y: scaled_y as u16,
                                pressed: false,
                            });
                        }
                    } else {
                        // No start recorded - just forward release
                        app_shell.queue_input(InputEvent::Touch {
                            x: scaled_x as u16,
                            y: scaled_y as u16,
                            pressed: false,
                        });
                    }
                    
                    // Reset swipe tracking
                    swipe_start_x = None;
                    swipe_start_y = None;
                    swipe_start_time = None;
                }
            }
        }

        // No automatic demo messages in manual mode

        // Process queued input events
        app_shell.process_input();

        // Process queued messages between apps
        app_shell.process_messages();

        // Process compositor commands from apps
        let commands = app_shell.take_compositor_commands();
        for command in commands {
            use crate::system::compositor::CompositorCommand;
            match command {
                CompositorCommand::SwitchToWindow { window_id, transition, duration_ms, easing } => {
                    compositor.switch_to_window(window_id, transition, duration_ms, easing);
                    // Update focus to match window
                    for (idx, &win_id) in windows.iter().enumerate() {
                        if win_id == window_id && idx < app_ids.len() {
                            app_shell.focus_app(app_ids[idx]);
                            break;
                        }
                    }
                }
                CompositorCommand::SwitchToApp { app_name, transition, duration_ms, easing } => {
                    if let Some(app_id) = app_shell.get_app_id_by_name(&app_name) {
                        // Find window for this app
                        if let Some(app) = app_shell.apps().iter().find(|a| a.info.id == app_id) {
                            if let Some(win_id) = app.info.window_id {
                                compositor.switch_to_window(win_id, transition, duration_ms, easing);
                                app_shell.focus_app(app_id);
                            }
                        }
                    }
                }
                CompositorCommand::SwitchNext { transition, duration_ms, easing } => {
                    // Find current window index
                    if let Some(current_win) = compositor.current_window() {
                        if let Some(current_idx) = windows.iter().position(|&w| w == current_win) {
                            // Switch to next window (wrapping around)
                            let next_idx = (current_idx + 1) % windows.len();
                            compositor.switch_to_window(windows[next_idx], transition, duration_ms, easing);
                            if next_idx < app_ids.len() {
                                app_shell.focus_app(app_ids[next_idx]);
                            }
                        }
                    }
                }
                CompositorCommand::SwitchPrevious { transition, duration_ms, easing } => {
                    // Find current window index
                    if let Some(current_win) = compositor.current_window() {
                        if let Some(current_idx) = windows.iter().position(|&w| w == current_win) {
                            // Switch to previous window (wrapping around)
                            let prev_idx = if current_idx == 0 {
                                windows.len() - 1
                            } else {
                                current_idx - 1
                            };
                            compositor.switch_to_window(windows[prev_idx], transition, duration_ms, easing);
                            if prev_idx < app_ids.len() {
                                app_shell.focus_app(app_ids[prev_idx]);
                            }
                        }
                    }
                }
            }
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
            
            // Log app performance metrics every 60 frames
            for (app_id, name, metrics) in app_shell.all_metrics() {
                info!(
                    "App[{}] service={}us ui_avg={}us ui_last={}us frames={}",
                    name,
                    metrics.service_time_us,
                    metrics.avg_frame_time_us,
                    metrics.last_frame_time_us,
                    metrics.frame_count
                );
            }
        }

        frame_counter = frame_counter.wrapping_add(1);
        Timer::after(Duration::from_millis(16)).await;
    }
}
