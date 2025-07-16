#![allow(unused)]

use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::window::{Window, WindowHandle};

use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use libm::sqrtf;

use crate::system::resources::framebuffer::{FRAMEBUFFER_POOL, FrameBufferHandle};

// Screen constants matching your pool
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 64;
const FRAME_BUFFER_SIZE: usize = 256; // matches your pool FRAME_BUFFER_SIZE

// Animation tuning globals
const ANIM_STEPS: usize = 16;
const ANIM_FRAME_DELAY_MS: u64 = 20;

#[derive(Clone, Copy, Debug)]
pub enum ViewMode {
    Single,
    Split,
}

#[derive(Clone, Copy)]
pub enum SlideDir {
    Left,
    Right,
}

pub struct UICompositor {
    windows: Vec<Window, 8>,
    current_index: usize,
    view_mode: ViewMode,
    composited_handle: Option<FrameBufferHandle>,
    display: Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
    redraw_requests: Vec<WindowHandle, 4>,
}

impl UICompositor {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_index: 0,
            view_mode: ViewMode::Single,
            composited_handle: None,
            display: None,
            redraw_requests: Vec::new(),
        }
    }

    pub fn attach_display(
        &mut self,
        display: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>,
    ) {
        self.display = Some(display);
    }

    /// External callers request redraw by handle
    pub fn request_redraw(&mut self, handle: WindowHandle) {
        if !self.redraw_requests.contains(&handle) {
            self.redraw_requests.push(handle).ok();
        }
    }

    /// Run a frame: if redraw requested, composite and draw
    pub async fn step(&mut self) {
        if self.redraw_requests.is_empty() {
            return;
        }

        if let Some(display) = self.display {
            if let Some(frame) = self.composite().await {
                let mut disp = display.lock().await;
                disp.draw(frame).await;
                self.release_last();
            }
        }

        self.redraw_requests.clear();
    }

    /// Return the handle of the currently focused window
    pub fn current_handle(&self) -> WindowHandle {
        self.windows[self.current_index].handle()
    }

    /// Find a mutable reference to a window by its handle (for external callers)
    pub fn window_for_handle_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }

    /// Allocate a new window and return its handle and canvas
    pub async fn alloc_window_with_canvas(
        &mut self,
        width: usize,
        height: usize,
        id: usize,
    ) -> Option<WindowHandle> {
        let window = Window::new(width, height, id).await;
        let handle = window.handle();

        self.windows.push(window).ok()?;

        // Now get a mutable ref to the just pushed window (last element)
        let window = self.windows.last_mut()?;
        let canvas = window.canvas().await;

        Some(handle)
    }

    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Single => ViewMode::Split,
            ViewMode::Split => ViewMode::Single,
        };
    }

    pub fn next_window(&mut self) {
        if !self.windows.is_empty() {
            self.current_index = (self.current_index + 1) % self.windows.len();
        }
    }

    pub fn prev_window(&mut self) {
        if !self.windows.is_empty() {
            self.current_index = (self.current_index + self.windows.len() - 1) % self.windows.len();
        }
    }

    /// Animate sliding between windows using stack buffer (no framebuffer pool)
    pub async fn animate_slide(&mut self, dir: SlideDir) {
        if self.windows.len() < 2 {
            return;
        }

        let from_index = self.current_index;
        let to_index = match dir {
            SlideDir::Left => {
                if self.current_index == 0 {
                    self.windows.len() - 1
                } else {
                    self.current_index - 1
                }
            }
            SlideDir::Right => (self.current_index + 1) % self.windows.len(),
        };

        let mut composed_buf = [0u8; FRAME_BUFFER_SIZE];

        for step in 0..=ANIM_STEPS {
            let t = step as f32 / ANIM_STEPS as f32;
            let eased = ease_in_out_circular(t);
            let offset = (eased * SCREEN_WIDTH as f32) as i32;

            composed_buf.fill(0);

            let (from_x, to_x) = match dir {
                SlideDir::Left => {
                    let from_x = -offset;
                    let to_x = from_x + SCREEN_WIDTH as i32;
                    (from_x, to_x)
                }
                SlideDir::Right => {
                    let from_x = offset;
                    let to_x = from_x - SCREEN_WIDTH as i32;
                    (from_x, to_x)
                }
            };

            // Use direct window references — no need to go via handle lookup
            let src1_win = &mut self.windows[from_index];
            let src1_canvas = src1_win.canvas().await;
            let src1_buf = src1_canvas.buffer();
            blit_into(
                &mut composed_buf,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
                src1_buf,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
                from_x,
                0,
            );

            let mut src2_win = &mut self.windows[to_index];
            let src2_canvas = src2_win.canvas().await;
            let src2_buf = src2_canvas.buffer();
            blit_into(
                &mut composed_buf,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
                src2_buf,
                SCREEN_WIDTH as u32,
                SCREEN_HEIGHT as u32,
                to_x,
                0,
            );

            if let Some(display) = self.display {
                let mut disp = display.lock().await;
                disp.draw(&composed_buf).await;
            }

            Timer::after(Duration::from_millis(ANIM_FRAME_DELAY_MS)).await;
        }

        self.current_index = to_index;
    }

    /// Compose current windows into a freshly allocated framebuffer from pool
    pub async fn composite(&mut self) -> Option<&'static [u8]> {
        let handle = FRAMEBUFFER_POOL.allocate().await?;
        let buf = FRAMEBUFFER_POOL.get_mut(&handle);
        buf.fill(0);

        match self.view_mode {
            ViewMode::Single => {
                let window = &mut self.windows[self.current_index];
                let src_canvas = window.canvas().await;
                let src_buf = src_canvas.buffer();
                blit_into(
                    buf,
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    src_buf,
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    0,
                    0,
                );
            }
            ViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.windows.len();

                let src1_win = &mut self.windows[i1];
                let src1_canvas = src1_win.canvas().await;
                blit_into(
                    buf,
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    src1_canvas.buffer(),
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    0,
                    0,
                );

                let src2_win = &mut self.windows[i2];
                let src2_canvas = src2_win.canvas().await;
                blit_into(
                    buf,
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    src2_canvas.buffer(),
                    SCREEN_WIDTH as u32,
                    SCREEN_HEIGHT as u32,
                    (SCREEN_WIDTH / 2) as i32,
                    0,
                );
            }
        }

        self.release_last(); // Release previous buffer if any
        self.composited_handle = Some(handle);

        Some(buf)
    }

    /// Release last composited framebuffer handle back to pool
    pub fn release_last(&mut self) {
        if let Some(handle) = self.composited_handle.take() {
            FRAMEBUFFER_POOL.release(&handle);
        }
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Check if a window handle is the currently focused window
    pub fn is_focused(&self, handle: WindowHandle) -> bool {
        self.windows[self.current_index].handle() == handle
    }

    /// Poll input event from a window's input receiver (if available)
    pub fn poll_input(
        &mut self,
        handle: WindowHandle,
    ) -> Option<crate::system::services::human_input_srv::HumanInputEvent> {
        self.window_for_handle_mut(handle)
            .and_then(|w| w.input_receiver().try_receive().ok())
    }
}

// Blit (copy) pixels from src buffer into dest buffer at x,y offset.
// Assumes 1 bit per pixel packed vertically in bytes.
fn blit_into(
    dest: &mut [u8],
    dest_width: u32,
    dest_height: u32,
    src: &[u8],
    src_width: u32,
    src_height: u32,
    x_off: i32,
    y_off: i32,
) {
    for y in 0..src_height {
        let dest_y = y as i32 + y_off;
        if dest_y < 0 || dest_y >= dest_height as i32 {
            continue;
        }
        for x in 0..src_width {
            let dest_x = x as i32 + x_off;
            if dest_x < 0 || dest_x >= dest_width as i32 {
                continue;
            }

            let src_byte_index = (x + (y / 8) * src_width) as usize;
            let bit_mask = 1 << (y % 8);

            if src[src_byte_index] & bit_mask != 0 {
                let dest_byte_index =
                    (dest_x as usize) + ((dest_y as usize / 8) * dest_width as usize);
                dest[dest_byte_index] |= 1 << (dest_y as usize % 8);
            } else {
                let dest_byte_index =
                    (dest_x as usize) + ((dest_y as usize / 8) * dest_width as usize);
                dest[dest_byte_index] &= !(1 << (dest_y as usize % 8));
            }
        }
    }
}

// === Easing function ===
fn ease_in_out_circular(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1.0 - sqrtf(1.0 - 4.0 * t * t))
    } else {
        0.5 * (sqrtf(1.0 - (2.0 * t - 2.0).powf(2.0)) + 1.0)
    }
}

use micromath::F32Ext;
