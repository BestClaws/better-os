//! App Shell / Spawner
//!
//! Manages application lifecycle: spawning, naming, ID assignment, and window allocation.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;

use crate::system::surface::Surface;
use crate::system::window_manager::{WindowId, WindowManager, WindowGeometry};

/// Unique identifier for an application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AppId(u32);

impl AppId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Application metadata
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub id: AppId,
    pub name: String,
    pub window_id: Option<WindowId>,
}

/// Trait that all applications must implement
pub trait App: Send {
    /// Initialize the app with its assigned surface
    fn init(&mut self, surface: &mut Surface);

    /// Update the app (called each frame)
    fn update(&mut self, surface: &mut Surface, delta_ms: u32);

    /// Handle app suspension (when window is hidden/backgrounded)
    fn suspend(&mut self) {}

    /// Handle app resume (when window becomes visible again)
    fn resume(&mut self) {}

    /// Get the app's name
    fn name(&self) -> &str;
}

/// Application instance with metadata
pub struct AppInstance {
    pub info: AppInfo,
    pub app: Box<dyn App>,
    pub active: bool,
}

/// The app shell manages application lifecycle
pub struct AppShell {
    apps: Vec<AppInstance>,
    next_id: u32,
    window_format: u8, // 0 = LUMA4, 2 = RGB565
}

impl AppShell {
    /// Create a new app shell with the specified window format
    pub fn new(window_format: u8) -> Self {
        Self {
            apps: Vec::new(),
            next_id: 1,
            window_format,
        }
    }

    /// Spawn a new application
    pub fn spawn_app(
        &mut self,
        name: String,
        app: Box<dyn App>,
        window_manager: &mut WindowManager,
        geometry: WindowGeometry,
    ) -> AppId {
        let id = AppId::new(self.next_id);
        self.next_id += 1;

        // Create a window for this app
        let window_id = window_manager.create_window(
            format!("{} Window", name),
            geometry,
            self.window_format,
        );

        // Initialize the app with its surface
        if let Some(window) = window_manager.get_window_mut(window_id) {
            let mut surface = if self.window_format == 2 {
                Surface::new_rgb565(
                    &mut window.frame_buffer,
                    geometry.width,
                    geometry.height,
                )
            } else {
                Surface::new_luma4(
                    &mut window.frame_buffer,
                    geometry.width,
                    geometry.height,
                )
            };
            
            let mut app_instance = AppInstance {
                info: AppInfo {
                    id,
                    name: name.clone(),
                    window_id: Some(window_id),
                },
                app,
                active: true,
            };

            app_instance.app.init(&mut surface);
            self.apps.push(app_instance);
        }

        id
    }

    /// Get an app by ID
    pub fn get_app(&self, id: AppId) -> Option<&AppInstance> {
        self.apps.iter().find(|a| a.info.id == id)
    }

    /// Get a mutable app by ID
    pub fn get_app_mut(&mut self, id: AppId) -> Option<&mut AppInstance> {
        self.apps.iter_mut().find(|a| a.info.id == id)
    }

    /// Get app by name
    pub fn get_app_by_name(&self, name: &str) -> Option<&AppInstance> {
        self.apps.iter().find(|a| a.info.name == name)
    }

    /// Terminate an app and destroy its window
    pub fn terminate_app(&mut self, id: AppId, window_manager: &mut WindowManager) -> bool {
        if let Some(pos) = self.apps.iter().position(|a| a.info.id == id) {
            let app = self.apps.remove(pos);
            if let Some(window_id) = app.info.window_id {
                window_manager.destroy_window(window_id);
            }
            true
        } else {
            false
        }
    }

    /// Update all active apps
    pub fn update_apps(&mut self, window_manager: &mut WindowManager, delta_ms: u32) {
        for app_instance in &mut self.apps {
            if !app_instance.active {
                continue;
            }

            if let Some(window_id) = app_instance.info.window_id {
                if let Some(window) = window_manager.get_window_mut(window_id) {
                    if window.info.visible {
                        let mut surface = if window.bytes_per_pixel == 2 {
                            Surface::new_rgb565(
                                &mut window.frame_buffer,
                                window.info.geometry.width,
                                window.info.geometry.height,
                            )
                        } else {
                            Surface::new_luma4(
                                &mut window.frame_buffer,
                                window.info.geometry.width,
                                window.info.geometry.height,
                            )
                        };
                        app_instance.app.update(&mut surface, delta_ms);
                        window.mark_dirty();
                    }
                }
            }
        }
    }

    /// Suspend an app
    pub fn suspend_app(&mut self, id: AppId) {
        if let Some(app) = self.get_app_mut(id) {
            app.active = false;
            app.app.suspend();
        }
    }

    /// Resume an app
    pub fn resume_app(&mut self, id: AppId) {
        if let Some(app) = self.get_app_mut(id) {
            app.active = true;
            app.app.resume();
        }
    }

    /// Get all apps
    pub fn apps(&self) -> &[AppInstance] {
        &self.apps
    }

    /// Get total number of apps
    pub fn app_count(&self) -> usize {
        self.apps.len()
    }
}
