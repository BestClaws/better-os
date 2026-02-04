//! Window Manager
//!
//! Manages windows and their associated frame buffers. Each window has a unique ID,
//! name, and backing frame buffer that applications can draw to.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;

/// Unique identifier for a window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(u32);

impl WindowId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Window dimensions and position
#[derive(Debug, Clone, Copy)]
pub struct WindowGeometry {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Window metadata
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: WindowId,
    pub name: String,
    pub geometry: WindowGeometry,
    pub visible: bool,
    pub z_order: u32,
}

/// A window with its backing frame buffer
pub struct Window {
    pub info: WindowInfo,
    /// Raw frame buffer data (format-specific, typically RGB565 or LUMA4)
    pub frame_buffer: Box<[u8]>,
    /// Bytes per pixel (2 for RGB565, 0.5 for LUMA4 packed)
    pub bytes_per_pixel: u8,
    /// Whether the window has been modified since last composite
    pub dirty: bool,
}

impl Window {
    /// Create a new window with the specified geometry and pixel format
    pub fn new(id: WindowId, name: String, geometry: WindowGeometry, bytes_per_pixel: u8) -> Self {
        let buffer_size = if bytes_per_pixel == 2 {
            // RGB565: 2 bytes per pixel
            (geometry.width as usize) * (geometry.height as usize) * 2
        } else {
            // LUMA4: 0.5 bytes per pixel (4 bits, packed)
            ((geometry.width as usize) * (geometry.height as usize) + 1) / 2
        };

        let frame_buffer = vec![0u8; buffer_size].into_boxed_slice();

        Self {
            info: WindowInfo {
                id,
                name,
                geometry,
                visible: true,
                z_order: 0,
            },
            frame_buffer,
            bytes_per_pixel,
            dirty: true,
        }
    }

    /// Mark window as needing redraw
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

/// The window manager maintains all windows and handles their lifecycle
pub struct WindowManager {
    windows: Vec<Window>,
    next_id: u32,
    next_z_order: u32,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            next_id: 1,
            next_z_order: 0,
        }
    }

    /// Create a new window
    pub fn create_window(
        &mut self,
        name: String,
        geometry: WindowGeometry,
        bytes_per_pixel: u8,
    ) -> WindowId {
        let id = WindowId::new(self.next_id);
        self.next_id += 1;

        let mut window = Window::new(id, name, geometry, bytes_per_pixel);
        window.info.z_order = self.next_z_order;
        self.next_z_order += 1;

        self.windows.push(window);
        id
    }

    /// Get a window by ID
    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.info.id == id)
    }

    /// Get a mutable window by ID
    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.info.id == id)
    }

    /// Get window by name
    pub fn get_window_by_name(&self, name: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.info.name == name)
    }

    /// Destroy a window
    pub fn destroy_window(&mut self, id: WindowId) -> bool {
        if let Some(pos) = self.windows.iter().position(|w| w.info.id == id) {
            self.windows.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all windows sorted by z-order (back to front)
    pub fn windows_by_z_order(&self) -> Vec<&Window> {
        let mut windows: Vec<&Window> = self.windows.iter().collect();
        windows.sort_by_key(|w| w.info.z_order);
        windows
    }

    /// Get all visible windows sorted by z-order
    pub fn visible_windows(&self) -> Vec<&Window> {
        let mut windows: Vec<&Window> = self
            .windows
            .iter()
            .filter(|w| w.info.visible)
            .collect();
        windows.sort_by_key(|w| w.info.z_order);
        windows
    }

    /// Get mutable references to all windows sorted by z-order
    pub fn windows_by_z_order_mut(&mut self) -> Vec<&mut Window> {
        let mut windows: Vec<&mut Window> = self.windows.iter_mut().collect();
        windows.sort_by_key(|w| w.info.z_order);
        windows
    }

    /// Bring window to front
    pub fn bring_to_front(&mut self, id: WindowId) {
        let next_z = self.next_z_order;
        self.next_z_order = next_z + 1;
        if let Some(window) = self.get_window_mut(id) {
            window.info.z_order = next_z;
            window.mark_dirty();
        }
    }

    /// Set window visibility
    pub fn set_visible(&mut self, id: WindowId, visible: bool) {
        if let Some(window) = self.get_window_mut(id) {
            window.info.visible = visible;
            window.mark_dirty();
        }
    }

    /// Get total number of windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Get all dirty windows
    pub fn dirty_windows(&self) -> Vec<&Window> {
        self.windows.iter().filter(|w| w.dirty).collect()
    }
}
