use crate::system::app::app_context::AppContext;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::window_manager::WindowManager;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use defmt::{debug, error, info, warn, Format};
use crate::apps::watch_app::watch_app;
use crate::apps::gfx_bench::gfx_bench_app;

/// Application registry for system apps
///
/// This defines the core applications that are automatically spawned
/// when the system starts. Each app gets its own window and context.
const SYSTEM_APPS: &[AppDescriptor] = &[
    AppDescriptor {
        name: "Watch",
        id: 1,
        spawn_fn: spawn_watch_app,
    },
    AppDescriptor {
        name: "GFX Benchmark",
        id: 2,
        spawn_fn: spawn_gfx_bench_app,
    },
];

/// Application descriptor for registration
struct AppDescriptor {
    /// Human-readable application name
    name: &'static str,
    /// Unique application identifier
    id: usize,
    /// Function to spawn the application task
    spawn_fn: fn(Spawner, AppContext) -> Result<(), embassy_executor::SpawnError>,
}

/// App spawner service - manages application lifecycle
///
/// This service is responsible for:
/// - Creating windows for each registered application
/// - Setting up application contexts with proper resources
/// - Spawning application tasks with the embassy executor
/// - Managing application lifecycle and error recovery
#[embassy_executor::task]
pub async fn app_spawner_service(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    spawner: Spawner,
) {
    info!("Starting application spawner service");

    let mut successful_apps = 0;
    let total_apps = SYSTEM_APPS.len();

    // Spawn each registered system application
    for app_descriptor in SYSTEM_APPS {
        debug!("Spawning app: {} (id={})", app_descriptor.name, app_descriptor.id);

        match spawn_application(&spawner, compositor, window_manager, app_descriptor).await {
            Ok(_) => {
                successful_apps += 1;
                info!("✓ App spawned: {}", app_descriptor.name);
            }
            Err(e) => {
                error!("✗ Failed to spawn {}: {:?}", app_descriptor.name, e);
            }
        }
    }

    info!("Application spawning complete: {}/{} apps started",
          successful_apps, total_apps);

    if successful_apps == 0 {
        error!("CRITICAL: No applications successfully started!");
    } else if successful_apps < total_apps {
        warn!("Some applications failed to start - system may have reduced functionality");
    }
}

/// Spawn a single application with proper error handling
async fn spawn_application(
    spawner: &Spawner,
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    app_descriptor: &AppDescriptor,
) -> Result<(), AppSpawnError> {
    // Create application context with window and resources
    let app_context = create_application_context(
        compositor,
        window_manager,
        app_descriptor.id,
        app_descriptor.name
    ).await?;

    // Spawn the application task
    (app_descriptor.spawn_fn)(*spawner, app_context)
        .map_err(AppSpawnError::TaskSpawnFailed)?;

    Ok(())
}

/// Create application context with allocated window and resources
///
/// This function:
/// - Allocates a new window in the compositor
/// - Creates an AppContext with the window handle
/// - Ensures proper resource allocation and error handling
pub async fn create_application_context(
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    app_id: usize,
    app_name: &'static str,
) -> Result<AppContext, AppSpawnError> {
    debug!("Creating context for app: {} (id={})", app_name, app_id);

    // Create window via WindowManager
    let window_handle = {
        let mut wm_lock = window_manager.lock().await;
        let handle = wm_lock
            .create_window(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT, app_id)
            .await
            .ok_or(AppSpawnError::WindowAllocationFailed)?;
        debug!("Window created for {}: {:?}", app_name, handle);
        handle
    };

    // Register window into compositor order
    let mut comp_lock = compositor.lock().await;
    let mut wm_for_register = window_manager.lock().await;
    comp_lock.register_window(&mut wm_for_register, window_handle).await;
    // Kick the compositor to render the first frame for this window soon
    comp_lock.request_redraw(window_handle);

    // Create application context
    let app_context = AppContext::new(window_handle, app_id, app_name, compositor, window_manager);

    Ok(app_context)
}

/// Errors that can occur during application spawning
#[derive(Debug, Format)]
pub enum AppSpawnError {
    /// Failed to allocate window in compositor
    WindowAllocationFailed,
    /// Failed to spawn application task
    TaskSpawnFailed(embassy_executor::SpawnError),
}

// Application spawn functions
// These wrapper functions provide type safety and error handling


// fn spawn_gfx_perf_bench_adaptive_app(spawner: Spawner, context: AppContext) -> Result<(), embassy_executor::SpawnError> {
//     spawner.spawn(gfx_perf_bench_adaptive_app(context))
// }

fn spawn_watch_app(spawner: Spawner, context: AppContext) -> Result<(), embassy_executor::SpawnError> {
    spawner.spawn(watch_app(context))
}

fn spawn_gfx_bench_app(spawner: Spawner, context: AppContext) -> Result<(), embassy_executor::SpawnError> {
    spawner.spawn(gfx_bench_app(context))
}
// Other app spawners removed for simplicity


/// Utility functions for application management
impl AppDescriptor {
    /// Get application info as formatted string
    pub fn info(&self) -> heapless::String<64> {
        let mut info = heapless::String::new();
        use heapless::String;
        use core::fmt::Write;

        write!(&mut info, "{} (id={})", self.name, self.id).ok();
        info
    }
}

/// Advanced application spawner for dynamic app loading
///
/// This allows spawning applications at runtime, useful for:
/// - Plugin systems
/// - Conditional app loading based on hardware
/// - User-installed applications
#[allow(dead_code)]
pub struct DynamicAppSpawner {
    compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
    window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    next_app_id: usize,
}

#[allow(dead_code)]
impl DynamicAppSpawner {
    pub fn new(
        compositor: &'static Mutex<CriticalSectionRawMutex, UICompositor>,
        window_manager: &'static Mutex<CriticalSectionRawMutex, WindowManager>,
    ) -> Self {
        Self {
            compositor,
            window_manager,
            next_app_id: 1000, // Start dynamic apps at high IDs
        }
    }

    /// Spawn application with automatically assigned ID
    pub async fn spawn_dynamic_app<F, E>(
        &mut self,
        spawner: &Spawner,
        app_name: &'static str,
        spawn_fn: F,
    ) -> Result<usize, AppSpawnError>
    where
        F: FnOnce(Spawner, AppContext) -> Result<(), E>,
        E: Into<embassy_executor::SpawnError>,
    {
        let app_id = self.next_app_id;
        self.next_app_id += 1;

        let context = create_application_context(
            self.compositor,
            self.window_manager,
            app_id,
            app_name,
        ).await?;

        spawn_fn(*spawner, context)
            .map_err(|e| AppSpawnError::TaskSpawnFailed(e.into()))?;

        info!("Dynamic app spawned: {} (id={})", app_name, app_id);
        Ok(app_id)
    }
}

/// System health monitoring for applications
#[cfg(feature = "app-health-monitoring")]
mod health_monitor {
    use super::*;
    use embassy_time::{Duration, Timer};
    use heapless::FnvIndexMap;

    /// Application health status
    #[derive(Debug, Clone, Copy)]
    pub enum AppHealth {
        Healthy,
        Unresponsive,
        Crashed,
    }

    /// Monitor application health and restart if needed
    pub struct AppHealthMonitor {
        app_status: FnvIndexMap<usize, AppHealth, 16>,
        restart_count: FnvIndexMap<usize, u32, 16>,
    }

    impl AppHealthMonitor {
        pub fn new() -> Self {
            Self {
                app_status: FnvIndexMap::new(),
                restart_count: FnvIndexMap::new(),
            }
        }

        /// Check and update application health
        pub fn check_app_health(&mut self, app_id: usize) -> AppHealth {
            // Implementation would check app responsiveness
            // For now, assume all apps are healthy
            *self.app_status.get(&app_id).unwrap_or(&AppHealth::Healthy)
        }

        /// Restart crashed application if restart limit not exceeded
        pub async fn restart_app_if_needed(
            &mut self,
            app_id: usize,
            max_restarts: u32,
        ) -> bool {
            let restart_count = self.restart_count.get(&app_id).copied().unwrap_or(0);

            if restart_count < max_restarts {
                self.restart_count.insert(app_id, restart_count + 1).ok();
                warn!("Restarting app {} (attempt {})", app_id, restart_count + 1);
                // Implementation would restart the app
                true
            } else {
                error!("App {} exceeded restart limit, disabling", app_id);
                false
            }
        }
    }

    /// Health monitoring service task
    #[embassy_executor::task]
    pub async fn app_health_monitor_service() {
        let mut monitor = AppHealthMonitor::new();

        loop {
            // Check all registered apps
            for app_descriptor in SYSTEM_APPS {
                let health = monitor.check_app_health(app_descriptor.id);

                match health {
                    AppHealth::Crashed => {
                        monitor.restart_app_if_needed(app_descriptor.id, 3).await;
                    }
                    AppHealth::Unresponsive => {
                        warn!("App {} is unresponsive", app_descriptor.name);
                    }
                    AppHealth::Healthy => {
                        // All good
                    }
                }
            }

            Timer::after(Duration::from_secs(30)).await;
        }
    }
}