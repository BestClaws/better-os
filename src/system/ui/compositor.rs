use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::window::{Window, WindowHandle};
use crate::system::ui::canvas::Canvas;
use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;
use libm::sqrtf;
use micromath::F32Ext;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_SIZE, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR};
use embedded_graphics::pixelcolor::{Gray4, GrayColor, PixelColor, Rgb565};
use embedded_graphics_core::prelude::DrawTarget;
use crate::system::resources::framebuffer::FRAMEBUFFER_POOL;
use crate::system::resources::input_channels::{INPUT_CHANNEL_POOL, CHANNEL_CAPACITY};

// Animation tuning globals
const ANIM_STEPS: usize = 6;
const ANIM_FRAME_DELAY_MS: u64 = 30;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    Single,
    Split,
}

#[derive(Clone, Copy)]
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
        }
    }

    /// Processes a single frame if redraw requests are pending.
    /// Composites the active windows and draws to the display.
    pub async fn step(&mut self) {
        if self.redraw_requests.is_empty() {
            return;
        }

        if let Some(display) = self.display {
            let mut working_buff = [0u8; FRAME_BUFFER_SIZE];
            let mut dest_canvas = Canvas::<Gray4>::new(FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);
            dest_canvas.set_resources(&mut working_buff);
            self.composite(&mut dest_canvas).await;
            let mut disp = display.lock().await;
            let then = Instant::now();
            disp.draw_gray4(dest_canvas.buffer_mut(), FRAME_SCALE_FACTOR).await;
            info!("frame time: {}", (Instant::now() - then).as_millis());
        }

        self.redraw_requests.clear();
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
        self.ensure_window_resources().await;
    }

    /// Ensures resources are allocated for the current, previous, and next windows.
    /// In split mode, also ensures resources for the second displayed window.
    /// Releases resources from all other windows to maintain pool limits.
    async fn ensure_window_resources(&mut self) {
        if self.windows.is_empty() {
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
                    window.relax();
                }
            }
        }

        // Allocate resources for prev, current, and next windows
        for &idx in &[prev_idx, curr_idx, next_idx] {
            let window = &mut self.windows[idx];
            if window.framebuffer_id().is_none() {
                // Allocate both resources as a pair
                if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                    if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                        window.set_resources(fb, ic).await;
                    } else {
                        // Release framebuffer if input channel allocation fails
                        FRAMEBUFFER_POOL.release(&fb);
                    }
                }
            }
        }

        // In split mode, ensure the next window has resources
        if let ViewMode::Split = self.view_mode {
            let split_next_idx = (self.current_window + 1) % len;
            let window = &mut self.windows[split_next_idx];
            if window.framebuffer_id().is_none() {
                if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                    if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                        window.set_resources(fb, ic).await;
                    } else {
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
            return;
        }

        let old_prev_idx = (self.current_window + self.windows.len() - 2) % self.windows.len();
        self.current_window = (self.current_window + 1) % self.windows.len();

        // Release resources for the old previous window first
        if self.windows.len() > 3 {
            let window = &mut self.windows[old_prev_idx];
            if window.framebuffer_id().is_some() {
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
            return;
        }

        let old_next_idx = (self.current_window + 2) % self.windows.len();
        self.current_window = (self.current_window + self.windows.len() - 1) % self.windows.len();

        // Release resources for the old next window first
        if self.windows.len() > 3 {
            let window = &mut self.windows[old_next_idx];
            if window.framebuffer_id().is_some() {
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
            return;
        }

        let from_index = self.current_window;
        let len = self.windows.len();
        let to_index = match dir {
            SlideDir::Left => (self.current_window + len - 1) % len,
            SlideDir::Right => (self.current_window + 1) % len,
        };

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
                window.relax();
            }
        }

        // Allocate resources for the target window
        let to_window = &mut self.windows[to_index];
        if to_window.framebuffer_id().is_none() {
            if let Some(fb) = FRAMEBUFFER_POOL.allocate().await {
                if let Some(ic) = INPUT_CHANNEL_POOL.allocate().await {
                    to_window.set_resources(fb, ic).await;
                } else {
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

        for step in 0..=ANIM_STEPS {
            let t = step as f32 / ANIM_STEPS as f32;
            let eased = ease_in_out_circular(t);
            let offset = (eased * FRAME_BUFFER_WIDTH as f32) as i32;

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

            let src1_canvas = self.windows[from_index].canvas().take().unwrap();
            let src2_canvas =  self.windows[to_index].canvas().take().unwrap();

            blit(&mut dest_canvas, &src1_canvas, from_x, 0);
            blit(&mut dest_canvas, &src2_canvas, to_x, 0);

            if let Some(display) = self.display {
                let mut disp = display.lock().await;
                disp.draw_gray4(dest_canvas.buffer_mut(), FRAME_SCALE_FACTOR).await;
            }

            Timer::after(Duration::from_millis(ANIM_FRAME_DELAY_MS)).await;
        }
    }

    /// Composites the active windows into the provided buffer.
    /// In single mode, renders the current window.
    /// In split mode, renders the current and next windows side by side.
    /// Skips rendering for windows without allocated resources.
    pub async fn composite<'a>(&'a mut self, working_buff: &mut Canvas<'a, Gray4>) {
        working_buff.clear(Gray4::BLACK).unwrap();

        let src_win = &mut self.windows[self.current_window];
        if let Some(src_canvas) = src_win.canvas() {
            blit(working_buff, &src_canvas, 0, 0);
        }

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