//! Compositor with window switching and animations
//!
//! Composites windows onto the display with smooth transitions between windows.

use embassy_time::Instant;
use gfx::colors::Color;
use gfx::rasterizer::RasterTarget;
use micromath::F32Ext;

// Derive Debug for TransitionType for logging

use crate::system::window_manager::{Window, WindowId, WindowManager};

/// Animation types for window transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionType {
    /// No animation, instant switch
    None,
    /// Fade in/out transition
    Fade,
    /// Slide from left
    SlideLeft,
    /// Slide from right
    SlideRight,
    /// Slide from top
    SlideTop,
    /// Slide from bottom
    SlideBottom,
    /// Scale/zoom transition
    Scale,
}

/// Easing functions for smooth animations
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Easing {
    /// Apply easing to a normalized time value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => t * (2.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }
}

/// State of an active transition
pub struct Transition {
    pub from_window: Option<WindowId>,
    pub to_window: WindowId,
    pub transition_type: TransitionType,
    pub easing: Easing,
    pub start_time: Instant,
    pub duration_ms: u32,
}

impl Transition {
    /// Get the current progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_millis() as u32;
        let t = (elapsed as f32 / self.duration_ms as f32).min(1.0);
        self.easing.apply(t)
    }

    /// Check if the transition is complete
    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed().as_millis() as u32 >= self.duration_ms
    }
}

/// The compositor combines windows and renders to the display
pub struct Compositor {
    /// Currently active window
    active_window: Option<WindowId>,
    /// Active transition, if any
    transition: Option<Transition>,
    /// Background color
    background_color: Color,
}

impl Compositor {
    /// Create a new compositor
    pub fn new() -> Self {
        Self {
            active_window: None,
            transition: None,
            background_color: Color::rgba(0, 0, 0, 255),
        }
    }

    /// Get the currently active window
    pub fn active_window(&self) -> Option<WindowId> {
        self.active_window
    }

    /// Set the background color
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    /// Switch to a window with animation
    pub fn switch_to_window(
        &mut self,
        window_id: WindowId,
        transition_type: TransitionType,
        duration_ms: u32,
        easing: Easing,
    ) {
        if self.active_window == Some(window_id) && self.transition.is_none() {
            return; // Already on this window
        }

        self.transition = Some(Transition {
            from_window: self.active_window,
            to_window: window_id,
            transition_type,
            easing,
            start_time: Instant::now(),
            duration_ms,
        });

        if transition_type == TransitionType::None {
            self.active_window = Some(window_id);
            self.transition = None;
        }
    }

    /// Switch to a window instantly
    pub fn switch_to_window_instant(&mut self, window_id: WindowId) {
        self.active_window = Some(window_id);
        self.transition = None;
    }

    /// Update and check if transition is complete
    pub fn update(&mut self) {
        if let Some(transition) = &self.transition {
            if transition.is_complete() {
                self.active_window = Some(transition.to_window);
                self.transition = None;
            }
        }
    }

    /// Composite windows to a display target
    pub fn composite<T: RasterTarget>(
        &self,
        target: &mut T,
        window_manager: &WindowManager,
    ) {
        // Clear background
        target.fill_solid_rect(0, 0, target.width(), target.height(), self.background_color);

        if let Some(transition) = &self.transition {
            // Render transition
            self.render_transition(target, window_manager, transition);
        } else if let Some(window_id) = self.active_window {
            // Render active window
            if let Some(window) = window_manager.get_window(window_id) {
                self.blit_window(target, window, 1.0, 0, 0);
            }
        }
    }

    /// Render a transition between windows
    fn render_transition<T: RasterTarget>(
        &self,
        target: &mut T,
        window_manager: &WindowManager,
        transition: &Transition,
    ) {
        let progress = transition.progress();

        match transition.transition_type {
            TransitionType::None => {
                // Should not happen
                if let Some(window) = window_manager.get_window(transition.to_window) {
                    self.blit_window(target, window, 1.0, 0, 0);
                }
            }
            TransitionType::Fade => {
                self.render_fade_transition(target, window_manager, transition, progress);
            }
            TransitionType::SlideLeft => {
                self.render_slide_transition(target, window_manager, transition, progress, -1, 0);
            }
            TransitionType::SlideRight => {
                self.render_slide_transition(target, window_manager, transition, progress, 1, 0);
            }
            TransitionType::SlideTop => {
                self.render_slide_transition(target, window_manager, transition, progress, 0, -1);
            }
            TransitionType::SlideBottom => {
                self.render_slide_transition(target, window_manager, transition, progress, 0, 1);
            }
            TransitionType::Scale => {
                self.render_scale_transition(target, window_manager, transition, progress);
            }
        }
    }

    /// Render fade transition
    fn render_fade_transition<T: RasterTarget>(
        &self,
        target: &mut T,
        window_manager: &WindowManager,
        transition: &Transition,
        progress: f32,
    ) {
        // Fade out old window
        if let Some(from_id) = transition.from_window {
            if let Some(window) = window_manager.get_window(from_id) {
                let alpha = 1.0 - progress;
                self.blit_window(target, window, alpha, 0, 0);
            }
        }

        // Fade in new window
        if let Some(window) = window_manager.get_window(transition.to_window) {
            self.blit_window(target, window, progress, 0, 0);
        }
    }

    /// Render slide transition
    fn render_slide_transition<T: RasterTarget>(
        &self,
        target: &mut T,
        window_manager: &WindowManager,
        transition: &Transition,
        progress: f32,
        dir_x: i32,
        dir_y: i32,
    ) {
        let width = target.width() as i32;
        let height = target.height() as i32;

        let offset_x = (dir_x as f32 * width as f32 * (1.0 - progress)) as i32;
        let offset_y = (dir_y as f32 * height as f32 * (1.0 - progress)) as i32;

        // Render old window sliding out
        if let Some(from_id) = transition.from_window {
            if let Some(window) = window_manager.get_window(from_id) {
                let old_offset_x = -(dir_x as f32 * width as f32 * progress) as i32;
                let old_offset_y = -(dir_y as f32 * height as f32 * progress) as i32;
                self.blit_window(target, window, 1.0, old_offset_x, old_offset_y);
            }
        }

        // Render new window sliding in
        if let Some(window) = window_manager.get_window(transition.to_window) {
            self.blit_window(target, window, 1.0, offset_x, offset_y);
        }
    }

    /// Render scale transition
    fn render_scale_transition<T: RasterTarget>(
        &self,
        target: &mut T,
        window_manager: &WindowManager,
        transition: &Transition,
        progress: f32,
    ) {
        // Scale out old window
        if let Some(from_id) = transition.from_window {
            if let Some(window) = window_manager.get_window(from_id) {
                let alpha = 1.0 - progress;
                self.blit_window(target, window, alpha, 0, 0);
            }
        }

        // Scale in new window (simplified - just fade for now)
        if let Some(window) = window_manager.get_window(transition.to_window) {
            self.blit_window(target, window, progress, 0, 0);
        }
    }

    /// Blit a window to the target with optional alpha and offset
    fn blit_window<T: RasterTarget>(
        &self,
        target: &mut T,
        window: &Window,
        alpha: f32,
        offset_x: i32,
        offset_y: i32,
    ) {
        let geom = window.info.geometry;
        let src_width = geom.width as usize;
        let src_height = geom.height as usize;

        // Detect window pixel format from bytes_per_pixel
        let is_rgb565 = window.bytes_per_pixel == 2;

        for y in 0..src_height {
            let target_y = (y as i32 + geom.y as i32 + offset_y) as u16;
            if target_y >= target.height() {
                continue;
            }

            for x in 0..src_width {
                let target_x = (x as i32 + geom.x as i32 + offset_x) as u16;
                if target_x >= target.width() {
                    continue;
                }

                let color = if is_rgb565 {
                    // Read RGB565 pixel (2 bytes per pixel)
                    let pixel_idx = (y * src_width + x) * 2;
                    if pixel_idx + 1 >= window.frame_buffer.len() {
                        continue;
                    }
                    let pixel_bytes = [window.frame_buffer[pixel_idx], window.frame_buffer[pixel_idx + 1]];
                    let pixel = u16::from_be_bytes(pixel_bytes);
                    
                    // Convert RGB565 to Color
                    let r = ((pixel >> 11) & 0x1F) as u8;
                    let g = ((pixel >> 5) & 0x3F) as u8;
                    let b = (pixel & 0x1F) as u8;
                    
                    Color::rgba(
                        (r << 3) | (r >> 2),
                        (g << 2) | (g >> 4),
                        (b << 3) | (b >> 2),
                        (alpha * 255.0) as u8,
                    )
                } else {
                    // Read LUMA4 pixel (0.5 bytes per pixel, packed)
                    let pixel_idx = (y * src_width + x) / 2;
                    if pixel_idx >= window.frame_buffer.len() {
                        continue;
                    }
                    let byte = window.frame_buffer[pixel_idx];
                    let nibble = if (x & 1) == 0 {
                        byte >> 4  // Even pixel: high nibble
                    } else {
                        byte & 0x0F  // Odd pixel: low nibble
                    };
                    
                    // Convert 4-bit grayscale to 8-bit (0-15 -> 0-255)
                    let gray = (nibble << 4) | nibble;
                    
                    Color::rgba(gray, gray, gray, (alpha * 255.0) as u8)
                };

                // Blend with alpha
                if alpha >= 0.99 {
                    target.fill_solid_hspan(target_y, target_x, color, 1);
                } else {
                    target.blend_solid_hspan(target_y, target_x, color, &[(alpha * 255.0) as u8]);
                }
            }
        }
    }
}
