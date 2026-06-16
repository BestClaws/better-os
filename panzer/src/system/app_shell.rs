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
use crate::system::input::{InputEvent, FocusEvent, LifecycleEvent};
use crate::system::compositor::CompositorCommand;

/// Unique identifier for an application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, defmt::Format)]
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

/// Inter-app message
#[derive(Debug, Clone)]
pub struct Message {
    pub from: AppId,
    pub to: AppId,
    pub data: Vec<u8>,
}

impl Message {
    /// Create a new message
    pub fn new(from: AppId, to: AppId, data: Vec<u8>) -> Self {
        Self { from, to, data }
    }

    /// Create a message from string data
    pub fn from_str(from: AppId, to: AppId, text: &str) -> Self {
        Self {
            from,
            to,
            data: text.as_bytes().to_vec(),
        }
    }

    /// Try to interpret data as UTF-8 string
    pub fn as_str(&self) -> Option<&str> {
        core::str::from_utf8(&self.data).ok()
    }
}

/// Trait that all applications must implement
pub trait App: Send {
    /// Initialize the app with its assigned surface
    fn init(&mut self, surface: &mut Surface);

    /// Update the app UI (called only when window is visible)
    fn update(&mut self, surface: &mut Surface, delta_ms: u32);

    /// Update background services (called every frame regardless of visibility)
    /// Use this for processing data, timers, sensors, network events, etc.
    fn service_update(&mut self, _delta_ms: u32) {}

    /// Handle input events (only called for focused app)
    /// Returns true if the input was handled, false otherwise
    fn on_input(&mut self, _event: InputEvent) -> bool {
        false
    }

    /// Handle focus changes
    fn on_focus(&mut self, _event: FocusEvent) {}

    /// Handle lifecycle events
    fn on_lifecycle(&mut self, _event: LifecycleEvent) {}

    /// Handle inter-app messages
    fn on_message(&mut self, _from: AppId, _data: &[u8]) {}

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
    pub metrics: AppMetrics,
}

/// Performance metrics for an application
#[derive(Debug, Clone, Copy)]
pub struct AppMetrics {
    /// Total CPU time spent in service_update (microseconds)
    pub service_time_us: u64,
    /// Total CPU time spent in update (microseconds)
    pub ui_time_us: u64,
    /// Number of frames rendered
    pub frame_count: u32,
    /// Average frame time (microseconds)
    pub avg_frame_time_us: u32,
    /// Last frame time (microseconds)
    pub last_frame_time_us: u32,
}

impl AppMetrics {
    pub fn new() -> Self {
        Self {
            service_time_us: 0,
            ui_time_us: 0,
            frame_count: 0,
            avg_frame_time_us: 0,
            last_frame_time_us: 0,
        }
    }

    pub fn record_service_time(&mut self, time_us: u32) {
        self.service_time_us += time_us as u64;
    }

    pub fn record_ui_time(&mut self, time_us: u32) {
        self.ui_time_us += time_us as u64;
        self.last_frame_time_us = time_us;
        self.frame_count += 1;
        
        // Update rolling average
        if self.frame_count > 0 {
            self.avg_frame_time_us = (self.ui_time_us / self.frame_count as u64) as u32;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

use crate::system::surface::DisplayInfo;

/// Focus manager to track which app has input focus
pub struct FocusManager {
    focused_app: Option<AppId>,
    focus_history: Vec<AppId>,
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focused_app: None,
            focus_history: Vec::new(),
        }
    }

    /// Get the currently focused app
    pub fn focused_app(&self) -> Option<AppId> {
        self.focused_app
    }

    /// Set focus to an app
    pub fn set_focus(&mut self, app_id: AppId) -> Option<AppId> {
        let previous = self.focused_app;
        
        if previous != Some(app_id) {
            self.focused_app = Some(app_id);
            
            // Add to history if not already there
            if !self.focus_history.contains(&app_id) {
                self.focus_history.push(app_id);
            }
        }
        
        previous
    }

    /// Clear focus
    pub fn clear_focus(&mut self) -> Option<AppId> {
        self.focused_app.take()
    }

    /// Focus the previous app from history
    pub fn focus_previous(&mut self) -> Option<AppId> {
        if self.focus_history.len() > 1 {
            // Remove current from history
            if let Some(current) = self.focused_app {
                self.focus_history.retain(|&id| id != current);
            }
            
            // Focus the last app in history
            if let Some(&prev_id) = self.focus_history.last() {
                self.focused_app = Some(prev_id);
                return Some(prev_id);
            }
        }
        None
    }
}

/// The app shell manages application lifecycle
pub struct AppShell {
    apps: Vec<AppInstance>,
    next_id: u32,
    window_format: u8, // 0 = LUMA4, 2 = RGB565
    display_info: DisplayInfo,
    focus_manager: FocusManager,
    input_queue: Vec<InputEvent>,
    message_queue: Vec<Message>,
    compositor_commands: Vec<CompositorCommand>,
}

impl AppShell {
    /// Create a new app shell with the specified window format and display info
    pub fn new(window_format: u8, display_info: DisplayInfo) -> Self {
        Self {
            apps: Vec::new(),
            next_id: 1,
            window_format,
            display_info,
            focus_manager: FocusManager::new(),
            input_queue: Vec::new(),
            message_queue: Vec::new(),
            compositor_commands: Vec::new(),
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
                    self.display_info,
                )
            } else {
                Surface::new_luma4(
                    &mut window.frame_buffer,
                    geometry.width,
                    geometry.height,
                    self.display_info,
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
                metrics: AppMetrics::new(),
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

    /// Update all active apps: background services always run, UI only when visible
    pub fn update_apps(&mut self, window_manager: &mut WindowManager, delta_ms: u32) {
        use embassy_time::Instant;
        
        // Collect window requests first to avoid borrow conflicts
        let mut pending_requests: Vec<(WindowId, Vec<crate::system::surface::WindowRequest>)> = Vec::new();
        
        for app_instance in &mut self.apps {
            if !app_instance.active {
                continue;
            }

            // Measure service update time
            let service_start = Instant::now();
            app_instance.app.service_update(delta_ms);
            let service_time_us = service_start.elapsed().as_micros() as u32;
            app_instance.metrics.record_service_time(service_time_us);

            // Only update UI if window is visible
            if let Some(window_id) = app_instance.info.window_id {
                if let Some(window) = window_manager.get_window_mut(window_id) {
                    if window.info.visible {
                        let ui_start = Instant::now();
                        
                        let mut surface = if window.bytes_per_pixel == 2 {
                            Surface::new_rgb565(
                                &mut window.frame_buffer,
                                window.info.geometry.width,
                                window.info.geometry.height,
                                self.display_info,
                            )
                        } else {
                            Surface::new_luma4(
                                &mut window.frame_buffer,
                                window.info.geometry.width,
                                window.info.geometry.height,
                                self.display_info,
                            )
                        };
                        
                        app_instance.app.update(&mut surface, delta_ms);
                        
                        // Get dirty regions from surface and mark on window
                        let dirty_regions = surface.take_dirty_regions();
                        // Get window requests before we borrow window again
                        let window_requests = surface.take_window_requests();
                        
                        // Only mark regions that were actually modified by the app
                        for region in dirty_regions {
                            window.mark_dirty_region(region);
                        }
                        
                        // Store requests for processing after the loop
                        if !window_requests.is_empty() {
                            pending_requests.push((window_id, window_requests));
                        }
                        
                        let ui_time_us = ui_start.elapsed().as_micros() as u32;
                        app_instance.metrics.record_ui_time(ui_time_us);
                    }
                }
            }
        }
        
        // Process window requests after the app update loop to avoid borrow conflicts
        for (window_id, requests) in pending_requests {
            for request in requests {
                self.process_window_request(window_id, request, window_manager);
            }
        }
    }

    /// Process a window property request
    fn process_window_request(
        &mut self,
        window_id: WindowId,
        request: crate::system::surface::WindowRequest,
        window_manager: &mut WindowManager,
    ) {
        use crate::system::surface::WindowRequest;
        
        match request {
            WindowRequest::SetVisible(visible) => {
                if let Some(window) = window_manager.get_window_mut(window_id) {
                    let was_visible = window.info.visible;
                    window.info.visible = visible;
                    window.mark_dirty();
                    
                    // Trigger lifecycle events
                    if was_visible != visible {
                        if let Some(app) = self.apps.iter_mut().find(|a| a.info.window_id == Some(window_id)) {
                            let event = if visible {
                                LifecycleEvent::Visible
                            } else {
                                LifecycleEvent::Hidden
                            };
                            app.app.on_lifecycle(event);
                        }
                    }
                }
            }
            WindowRequest::SetGeometry(geometry) => {
                if let Some(window) = window_manager.get_window_mut(window_id) {
                    window.info.geometry = geometry;
                    window.mark_dirty();
                }
            }
            WindowRequest::SetTitle(title) => {
                if let Some(window) = window_manager.get_window_mut(window_id) {
                    window.info.name = title;
                }
            }
            WindowRequest::BringToFront => {
                window_manager.bring_to_front(window_id);
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

    /// Queue an input event to be processed
    pub fn queue_input(&mut self, event: InputEvent) {
        self.input_queue.push(event);
    }

    /// Process queued input events and route to focused app
    pub fn process_input(&mut self) {
        if let Some(focused_id) = self.focus_manager.focused_app() {
            // Drain the queue into a temporary vector to avoid borrow conflicts
            let events: Vec<InputEvent> = self.input_queue.drain(..).collect();
            
            for event in events {
                if let Some(app) = self.get_app_mut(focused_id) {
                    app.app.on_input(event);
                }
            }
        } else {
            // No focused app, clear queue
            self.input_queue.clear();
        }
    }

    /// Set focus to an app
    pub fn focus_app(&mut self, app_id: AppId) {
        let previous = self.focus_manager.set_focus(app_id);
        
        // Send focus lost to previous app
        if let Some(prev_id) = previous {
            if prev_id != app_id {
                if let Some(app) = self.get_app_mut(prev_id) {
                    app.app.on_focus(FocusEvent::Lost);
                }
            }
        }
        
        // Send focus gained to new app
        if let Some(app) = self.get_app_mut(app_id) {
            app.app.on_focus(FocusEvent::Gained);
        }
    }

    /// Clear focus from all apps
    pub fn clear_focus(&mut self) {
        if let Some(prev_id) = self.focus_manager.clear_focus() {
            if let Some(app) = self.get_app_mut(prev_id) {
                app.app.on_focus(FocusEvent::Lost);
            }
        }
    }

    /// Get the currently focused app ID
    pub fn focused_app(&self) -> Option<AppId> {
        self.focus_manager.focused_app()
    }

    /// Send a message to a specific app
    pub fn send_message(&mut self, from: AppId, to: AppId, data: Vec<u8>) {
        self.message_queue.push(Message::new(from, to, data));
    }

    /// Send a message with string data
    pub fn send_str_message(&mut self, from: AppId, to: AppId, text: &str) {
        self.message_queue.push(Message::from_str(from, to, text));
    }

    /// Broadcast a message to all apps except sender
    pub fn broadcast_message(&mut self, from: AppId, data: Vec<u8>) {
        for app in &self.apps {
            if app.info.id != from {
                self.message_queue.push(Message::new(from, app.info.id, data.clone()));
            }
        }
    }

    /// Process queued messages and deliver to apps
    pub fn process_messages(&mut self) {
        // Drain messages into temporary vector to avoid borrow conflicts
        let messages: Vec<Message> = self.message_queue.drain(..).collect();
        
        for message in messages {
            if let Some(app) = self.get_app_mut(message.to) {
                app.app.on_message(message.from, &message.data);
            }
        }
    }

    /// Get app ID by name
    pub fn get_app_id_by_name(&self, name: &str) -> Option<AppId> {
        self.apps
            .iter()
            .find(|app| app.info.name == name)
            .map(|app| app.info.id)
    }

    /// Queue a compositor command
    pub fn queue_compositor_command(&mut self, command: CompositorCommand) {
        self.compositor_commands.push(command);
    }

    /// Get and clear compositor commands
    pub fn take_compositor_commands(&mut self) -> Vec<CompositorCommand> {
        core::mem::take(&mut self.compositor_commands)
    }

    /// Get app metrics by ID
    pub fn get_metrics(&self, id: AppId) -> Option<&AppMetrics> {
        self.apps
            .iter()
            .find(|a| a.info.id == id)
            .map(|a| &a.metrics)
    }

    /// Reset metrics for an app
    pub fn reset_metrics(&mut self, id: AppId) {
        if let Some(app) = self.get_app_mut(id) {
            app.metrics.reset();
        }
    }

    /// Get all app metrics
    pub fn all_metrics(&self) -> Vec<(AppId, &str, &AppMetrics)> {
        self.apps
            .iter()
            .map(|a| (a.info.id, a.info.name.as_str(), &a.metrics))
            .collect()
    }
}
