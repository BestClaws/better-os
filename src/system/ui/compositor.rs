use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::window::{Window, WindowHandle};
use crate::system::ui::canvas::Canvas;
use alloc::boxed::Box;
use defmt::{info, debug, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
use libm::sqrtf;
use micromath::F32Ext;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_SIZE, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR};
use embedded_graphics::pixelcolor::{Gray4, GrayColor, PixelColor, Rgb565};
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics_core::prelude::DrawTarget;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::resources::input_channels::{INPUT_CHANNEL_POOL, CHANNEL_CAPACITY};

// Animation tuning globals
const ANIM_STEPS: usize = 6;
const ANIM_FRAME_DELAY_MS: u64 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Format)]
pub enum ViewMode {
    Single,
    Split,
}

#[derive(Clone, Copy, Format)]
pub enum SlideDir {
    Left,
    Right,
}

/// Trait to handle pixel copying for different pixel color types.
trait BlitPixel: PixelColor {
    /// Copies a single pixel from source to destination buffer at the specified indices.
    fn blit_pixel(src: &[u8], src_pixel_idx: usize, dest: &mut [u8], dest_pixel_idx: usize) -> bool;
}

// Implementation for Rgb565 (2 bytes per pixel)
impl BlitPixel for Rgb565 {
    fn blit_pixel(src: &[u8], src_pixel_idx: usize, dest: &mut [u8], dest_pixel_idx: usize) -> bool {
        let src_idx = src_pixel_idx * 2;
        let dest_idx = dest_pixel_idx * 2;
        if src_idx + 1 < src.len() && dest_idx + 1 < dest.len() {
            dest[dest_idx] = src[src_idx];
            dest[dest_idx + 1] = src[src_idx + 1];
            true
        } else {
            false
        }
    }
}

// Implementation for Gray4 (4 bits per pixel, two pixels per byte)
impl BlitPixel for Gray4 {
    fn blit_pixel(src: &[u8], src_pixel_idx: usize, dest: &mut [u8], dest_pixel_idx: usize) -> bool {
        let src_byte_idx = src_pixel_idx / 2;
        let dest_byte_idx = dest_pixel_idx / 2;
        let src_is_high_nibble = (src_pixel_idx % 2) == 0;
        let dest_is_high_nibble = (dest_pixel_idx % 2) == 0;

        if src_byte_idx < src.len() && dest_byte_idx < dest.len() {
            let src_value = if src_is_high_nibble {
                (src[src_byte_idx] >> 4) & 0x0F // High nibble
            } else {
                src[src_byte_idx] & 0x0F // Low nibble
            };

            if dest_is_high_nibble {
                dest[dest_byte_idx] = (dest[dest_byte_idx] & 0x0F) | (src_value << 4);
            } else {
                dest[dest_byte_idx] = (dest[dest_byte_idx] & 0xF0) | src_value;
            }
            true
        } else {
            false
        }
    }
}

/// Blits a source Canvas to a destination Canvas with offsets.
/// Generic over pixel color type C, which must implement BlitPixel.
pub fn blit<'a, C: BlitPixel>(
    dest: &mut Canvas<'a, C>,
    src: &Canvas<'a, C>,
    x_off: i32,
    y_off: i32,
) {
    let start = Instant::now();
    let dest_width = dest.width();
    let dest_height = dest.height();
    let src_width = src.width();
    let src_height = src.height();

    for y in 0..src_height as i32 {
        let dest_y = y + y_off;
        if dest_y < 0 || dest_y >= dest_height as i32 {
            continue;
        }

        for x in 0..src_width as i32 {
            let dest_x = x + x_off;
            if dest_x < 0 || dest_x >= dest_width as i32 {
                continue;
            }

            let src_pixel_idx = y as usize * src_width as usize + x as usize;
            let dest_pixel_idx = dest_y as usize * dest_width as usize + dest_x as usize;

            C::blit_pixel(src.buffer(), src_pixel_idx, dest.buffer_mut(), dest_pixel_idx);
        }
    }
    debug!("blit full canvas: {} us", start.elapsed().as_micros());
}

/// Blits a specific region of a source Canvas to a destination Canvas with offsets.
pub fn blit_region<'a, C: BlitPixel>(
    dest: &mut Canvas<'a, C>,
    src: &Canvas<'a, C>,
    region: Rectangle,
    x_off: i32,
    y_off: i32,
) {
    let start = Instant::now();
    let dest_width = dest.width();
    let dest_height = dest.height();
    let src_width = src.width();
    let src_height = src.height();

    let (x0, y0, x1, y1) = (
        region.top_left.x.max(0) as u32,
        region.top_left.y.max(0) as u32,
        (region.top_left.x as u32 + region.size.width).min(src_width),
        (region.top_left.y as u32 + region.size.height).min(src_height),
    );

    if x0 >= x1 || y0 >= y1 {
        debug!("blit_region skipped: empty region x0={}, y0={}, x1={}, y1={}", x0, y0, x1, y1);
        return;
    }

    debug!("blit_region: x0={}, y0={}, width={}, height={}", x0, y0, x1 - x0, y1 - y0);

    for y in y0..y1 {
        let dest_y = y as i32 + y_off;
        if dest_y < 0 || dest_y >= dest_height as i32 {
            continue;
        }

        for x in x0..x1 {
            let dest_x = x as i32 + x_off;
            if dest_x < 0 || dest_x >= dest_width as i32 {
                continue;
            }

            let src_pixel_idx = y as usize * src_width as usize + x as usize;
            let dest_pixel_idx = dest_y as usize * dest_width as usize + dest_x as usize;

            C::blit_pixel(src.buffer(), src_pixel_idx, dest.buffer_mut(), dest_pixel_idx);
        }
    }
    debug!("blit_region completed: {} us", start.elapsed().as_micros());
}

pub struct UICompositor {
    windows: Vec<Window, 8>,
    current_window: usize,
    view_mode: ViewMode,
    display: Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
    redraw_requests: Vec<WindowHandle, 4>,
}

impl UICompositor {
    /// Creates a new UI compositor with no windows or display attached.
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_window: 0,
            view_mode: ViewMode::Single,
            display: None,
            redraw_requests: Vec::new(),
        }
    }

    /// Attaches a display to the compositor for rendering.
    pub fn attach_display(
        &mut self,
        display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) {
        self.display = Some(display);
    }

    /// Requests a redraw for a specific window by its handle.
    /// Ignores duplicate requests to avoid redundant redraws.
    pub fn request_redraw(&mut self, handle: WindowHandle) {
        if !self.redraw_requests.contains(&handle) {
            self.redraw_requests.push(handle).ok();
            debug!("Redraw requested for window {:?}", handle);
        }
    }

    /// Processes a single frame if redraw requests are pending.
    /// Composites the active windows and draws to the display.
    pub async fn step(&mut self) {
        let start = Instant::now();
        if self.redraw_requests.is_empty() {
            debug!("step skipped: no redraw requests");
            return;
        }

        debug!("step started with {} redraw requests", self.redraw_requests.len());

        if let Some(display) = self.display {
            let mut working_buff = [0u8; FRAME_BUFFER_SIZE];
            let mut dest_canvas = Canvas::<Gray4>::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);
            dest_canvas.set_resources(&mut working_buff);

            // Fetch view_mode and dirty region before borrowing self mutably
            let view_mode = self.view_mode;
            let dirty_region = self.windows[self.current_window]
                .canvas()
                .as_mut()
                .and_then(|canvas| canvas.dirty_region());

            if let Some(region) = dirty_region {
                debug!(
                    "Dirty region detected: x={}, y={}, width={}, height={}",
                    region.top_left.x, region.top_left.y, region.size.width, region.size.height
                );
            } else {
                debug!("No dirty region, full canvas will be used");
            }

            let composite_start = Instant::now();
            self.composite(&mut dest_canvas).await;
            debug!("Composite time: {} us", composite_start.elapsed().as_micros());

            let mut disp = display.lock().await;
            let draw_start = Instant::now();

            if view_mode == ViewMode::Single && dirty_region.is_some() {
                debug!("Drawing dirty region in single view mode");
                let region = dirty_region.unwrap();
                disp.draw_gray4_region(
                    dest_canvas.buffer(),
                    region,
                    FRAME_SCALE_FACTOR,
                ).await;
                // Flush the dirty region after drawing
                if let Some(canvas) = self.windows[self.current_window].canvas().as_mut() {
                    debug!("Flushing dirty region for window {}", self.current_window);
                    canvas.flush();
                }
            } else {
                debug!("Drawing full canvas (view_mode={:?})", view_mode);
                disp.draw_gray4(dest_canvas.buffer(), FRAME_SCALE_FACTOR).await;
            }

            debug!("Draw time: {} us", draw_start.elapsed().as_micros());
            debug!("Frame time: {} us", start.elapsed().as_micros());
        }

        self.redraw_requests.clear();
        debug!("Redraw requests cleared");
    }

    /// Returns the handle of the currently focused window, if any.
    pub fn current_handle(&self) -> Option<WindowHandle> {
        if !self.windows.is_empty() {
            Some(self.windows[self.current_window].handle())
        } else {
            None
        }
    }

    /// Retrieves a mutable reference to a window by its handle.
    pub fn get_window_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }

    /// Creates a new window with the specified dimensions and ID.
    /// Resources are not allocated until the window becomes active (prev, current, or next).
    pub async fn new_window(
        &mut self,
        width: u32,
        height: u32,
        id: usize,
    ) -> Option<WindowHandle> {
        let window = Window::new(width, height, id).await;
        let handle = window.handle();
        self.windows.push(window).ok()?;
        debug!("New window created: id={}, width={}, height={}", id, width, height);
        // Allocate resources for initial set of windows if this is the first window
        if self.windows.len() == 1 {
            self.ensure_window_resources().await;
        }
        Some(handle)
    }

    /// Toggles between single and split view modes.
    /// Ensures resources are allocated for the active windows in the new mode.
    pub async fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Single => ViewMode::Split,
            ViewMode::Split => ViewMode::Single,
        };
        debug!("View mode toggled to {:?}", self.view_mode);
        self.ensure_window_resources().await;
    }

    /// Ensures resources are allocated for the current, previous, and next windows.
    /// In split mode, also ensures resources for the second displayed window.
    /// Releases resources from all other windows to maintain pool limits.
    async fn ensure_window_resources(&mut self) {
        if self.windows.is_empty() {
            debug!("No windows to allocate resources for");
            return;
        }

        // Calculate indices for prev, current, and next windows
        let len = self.windows.len();
        let prev_idx = (self.current_window + len - 1) % len;
        let curr_idx = self.current_window;
        let next_idx = (self.current_window + 1) % len;

        // Release resources for windows that are not prev, current, or next
        for (i, window) in self.windows.iter_mut().enumerate() {
            if i != prev_idx && i != curr_idx && i != next_idx {
                if window.framebuffer_id().is_some() {
                    debug!("Releasing resources for window {}", i);
                    window.relax();
                }
            }
        }

        // Allocate resources for prev, current, and next windows
        for &idx in &[prev_idx, curr_idx, next_idx] {
            let window = &mut self.windows[idx];
            if window.framebuffer_id().is_none() {
                debug!("Allocating resources for window {}", idx);
                if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                    if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                        window.set_resources(fb, ic).await;
                    } else {
                        debug!("Failed to allocate input channel, releasing framebuffer");
                        FRAMEBUFFER_POOL.release(&fb);
                    }
                } else {
                    debug!("Failed to allocate framebuffer for window {}", idx);
                }
            }
        }

        // In split mode, ensure the next window has resources
        if let ViewMode::Split = self.view_mode {
            let split_next_idx = (self.current_window + 1) % len;
            let window = &mut self.windows[split_next_idx];
            if window.framebuffer_id().is_none() {
                debug!("Allocating resources for split mode window {}", split_next_idx);
                if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                    if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                        window.set_resources(fb, ic).await;
                    } else {
                        debug!("Failed to allocate input channel, releasing framebuffer");
                        FRAMEBUFFER_POOL.release(&fb);
                    }
                }
            }
        }
    }

    /// Switches to the next window in single mode.
    /// Does nothing in split mode or if no windows exist.
    /// Manages resource allocation for the new active set.
    pub async fn next_window(&mut self) {
        if self.windows.is_empty() || self.view_mode == ViewMode::Split {
            debug!("next_window skipped: empty or split mode");
            return;
        }

        let old_prev_idx = (self.current_window + self.windows.len() - 2) % self.windows.len();
        self.current_window = (self.current_window + 1) % self.windows.len();
        debug!("Switched to next window: {}", self.current_window);

        // Release resources for the old previous window first
        if self.windows.len() > 3 {
            let window = &mut self.windows[old_prev_idx];
            if window.framebuffer_id().is_some() {
                debug!("Releasing resources for old previous window {}", old_prev_idx);
                window.relax();
            }
        }

        // Allocate resources for new prev, current, and next windows
        self.ensure_window_resources().await;
    }

    /// Switches to the previous window in single mode.
    /// Does nothing in split mode or if no windows exist.
    /// Manages resource allocation for the new active set.
    pub async fn prev_window(&mut self) {
        if self.windows.is_empty() || self.view_mode == ViewMode::Split {
            debug!("prev_window skipped: empty or split mode");
            return;
        }

        let old_next_idx = (self.current_window + 2) % self.windows.len();
        self.current_window = (self.current_window + self.windows.len() - 1) % self.windows.len();
        debug!("Switched to previous window: {}", self.current_window);

        // Release resources for the old next window first
        if self.windows.len() > 3 {
            let window = &mut self.windows[old_next_idx];
            if window.framebuffer_id().is_some() {
                debug!("Releasing resources for old next window {}", old_next_idx);
                window.relax();
            }
        }

        // Allocate resources for new prev, current, and next windows
        self.ensure_window_resources().await;
    }

    /// Animates a slide transition to the next or previous window in single mode.
    /// Does nothing in split mode or if fewer than two windows exist.
    /// Ensures resources are allocated for both source and target windows before animating.
    pub async fn animate_slide(&mut self, dir: SlideDir) {
        if self.windows.len() < 2 || self.view_mode == ViewMode::Split {
            debug!("animate_slide skipped: too few windows ({}) or split mode", self.windows.len());
            return;
        }

        let from_index = self.current_window;
        let len = self.windows.len();
        let to_index = match dir {
            SlideDir::Left => (self.current_window + len - 1) % len,
            SlideDir::Right => (self.current_window + 1) % len,
        };

        debug!("Animating slide from window {} to {}", from_index, to_index);

        // Ensure resources for the target window before switching
        let old_prev_idx = (self.current_window + len - 2) % len;
        let old_next_idx = (self.current_window + 2) % len;
        let window_to_relax_idx = match dir {
            SlideDir::Left => old_next_idx,
            SlideDir::Right => old_prev_idx,
        };

        // Release resources for the window that will no longer be needed
        if self.windows.len() > 3 {
            let window = &mut self.windows[window_to_relax_idx];
            if window.framebuffer_id().is_some() {
                debug!("Releasing resources for window {}", window_to_relax_idx);
                window.relax();
            }
        }

        // Allocate resources for the target window
        let to_window = &mut self.windows[to_index];
        if to_window.framebuffer_id().is_none() {
            debug!("Allocating resources for target window {}", to_index);
            if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                    to_window.set_resources(fb, ic).await;
                } else {
                    debug!("Failed to allocate input channel, releasing framebuffer");
                    FRAMEBUFFER_POOL.release(&fb);
                }
            }
        }

        // Perform the window switch
        match dir {
            SlideDir::Left => self.prev_window().await,
            SlideDir::Right => self.next_window().await,
        }

        let mut composed_buf = [0u8; FRAME_BUFFER_SIZE];
        let mut dest_canvas: Canvas<Gray4> = Canvas::<Gray4>::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);
        dest_canvas.set_resources(&mut composed_buf);

        // Take the canvases upfront
        let src1_canvas = self.windows[from_index].canvas().take().expect("Source canvas not available");
        let src2_canvas = self.windows[to_index].canvas().take().expect("Target canvas not available");

        for step in 0..=ANIM_STEPS {
            let t = step as f32 / ANIM_STEPS as f32;
            let eased = ease_in_out_circular(t);
            let offset = (eased * FRAME_BUFFER_WIDTH as f32) as i32;

            let step_start = Instant::now();
            dest_canvas.clear(Gray4::BLACK).unwrap();

            let (from_x, to_x) = match dir {
                SlideDir::Left => {
                    let from_x = -offset;
                    let to_x = from_x + FRAME_BUFFER_WIDTH as i32;
                    (from_x, to_x)
                }
                SlideDir::Right => {
                    let from_x = offset;
                    let to_x = from_x - FRAME_BUFFER_WIDTH as i32;
                    (from_x, to_x)
                }
            };

            debug!("Animation step {}: from_x={}, to_x={}", step, from_x, to_x);
            blit(&mut dest_canvas, &src1_canvas, from_x, 0);
            blit(&mut dest_canvas, &src2_canvas, to_x, 0);

            if let Some(display) = self.display {
                let mut disp = display.lock().await;
                disp.draw_gray4(dest_canvas.buffer(), FRAME_SCALE_FACTOR).await;
            }

            debug!("Animation step {} time: {} us", step, step_start.elapsed().as_micros());
            Timer::after(Duration::from_millis(ANIM_FRAME_DELAY_MS)).await;
        }

        // Restore the canvases
        self.windows[from_index].canvas().replace(src1_canvas);
        self.windows[to_index].canvas().replace(src2_canvas);
    }

    /// Composites the active windows into the provided buffer.
    /// In single mode, renders only the dirty region of the current window if available.
    /// In split mode, renders the current and next windows side by side (full canvas).
    /// Skips rendering for windows without allocated resources.
    pub async fn composite<'a>(&'a mut self, working_buff: &mut Canvas<'a, Gray4>) {
        let start = Instant::now();
        working_buff.clear(Gray4::BLACK).unwrap();
        debug!("Cleared destination canvas");

        let src_win = &mut self.windows[self.current_window];
        if let Some(src_canvas) = src_win.canvas().as_mut() {
            if self.view_mode == ViewMode::Single {
                if let Some(dirty_region) = src_canvas.dirty_region() {
                    debug!("Compositing dirty region for window {}", self.current_window);
                    blit_region(working_buff, src_canvas, dirty_region, 0, 0);
                } else {
                    debug!("Compositing full canvas for window {}", self.current_window);
                    blit(working_buff, src_canvas, 0, 0);
                }
            } else {
                debug!("Compositing split mode: window {} and next", self.current_window);
                blit(working_buff, src_canvas, 0, 0);
                let next_idx = (self.current_window + 1) % self.windows.len();
                if let Some(next_canvas) = self.windows[next_idx].canvas().as_mut() {
                    debug!("Blitting next window {} at x_offset={}", next_idx, FRAME_BUFFER_WIDTH / 2);
                    blit(working_buff, next_canvas, FRAME_BUFFER_WIDTH as i32 / 2, 0);
                }
            }
        } else {
            debug!("No canvas available for window {}", self.current_window);
        }
        debug!("Composite total time: {} us", start.elapsed().as_micros());
    }

    /// Returns the current view mode (single or split).
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Checks if a window handle is the currently focused window.
    pub fn is_focused(&self, handle: WindowHandle) -> bool {
        !self.windows.is_empty() && self.windows[self.current_window].handle() == handle
    }

    /// Polls for an input event from a window's input receiver, if available.
    pub fn poll_input(
        &mut self,
        handle: WindowHandle,
    ) -> Option<crate::system::services::human_input_srv::HumanInputEvent> {
        self.get_window_mut(handle)
            .and_then(|w| w.input_receiver().and_then(|r| r.try_receive().ok()))
    }
}

/// Easing function for smooth animation transitions.
fn ease_in_out_circular(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - sqrtf(1.0 - 4.0 * t * t))
    } else {
        0.5 * (sqrtf(1.0 - (2.0 * t - 2.0).powf(2.0)) + 1.0)
    }
}
