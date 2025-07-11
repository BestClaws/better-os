#![allow(unused)]


use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::{allocate_buffer, get_buffer_slice, release_buffer};
use crate::system::ui::input_channels::allocate_channel;
use crate::system::ui::window::{Window, WindowHandle};

use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use libm::sqrtf;

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
    composited_id: Option<usize>,
    display: Option<&'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>>,
    redraw_requests: Vec<WindowHandle, 4>,
}

impl UICompositor {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_index: 0,
            view_mode: ViewMode::Single,
            composited_id: None,
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

    pub fn request_redraw(&mut self, handle: WindowHandle) {
        if !self.redraw_requests.contains(&handle) {
            self.redraw_requests.push(handle).ok();
        }
    }

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

    pub fn current_handle(&self) -> WindowHandle {
        self.windows.get(self.current_index).map(|w| w.handle()).unwrap()
    }

    pub fn window_for_handle_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }

    pub async fn alloc_window_with_canvas(
        &mut self,
        width: usize,
        height: usize,
        id: usize,
    ) -> Option<(WindowHandle, Canvas)> {
        let fb = allocate_buffer().await?;
        let input = allocate_channel().await?;

        let window = Window::new(fb, input, width, height, id);
        let handle = window.handle();

        self.windows.push(window).ok()?;
        let window = self.windows.iter_mut().find(|w| w.handle() == handle)?;
        let canvas = window.canvas();

        Some((handle, canvas))
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

        let from_handle = self.windows[from_index].handle();
        let to_handle = self.windows[to_index].handle();
        let screen_width = 128; // Screen width in pixels

        for step in 0..=ANIM_STEPS {
            let t = step as f32 / ANIM_STEPS as f32;
            let eased = ease_in_out_circular(t);
            let offset = (eased * screen_width as f32) as i32;

            let mut fb = allocate_buffer().await.unwrap();
            let id = fb.id();
            let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
            composed.clear();

            // Calculate positions
            let (from_x, to_x) = match dir {
                SlideDir::Left => {
                    let from_x = -offset; // Outgoing moves left
                    let to_x = from_x + screen_width; // Incoming follows right edge
                    (from_x, to_x)
                }
                SlideDir::Right => {
                    let from_x = offset; // Outgoing moves right
                    let to_x = from_x - screen_width; // Incoming follows left edge
                    (from_x, to_x)
                }
            };

            // Draw windows, ensuring we handle negative coordinates safely
            if let Some(w1) = self.window_for_handle_mut(from_handle) {
                let canvas1 = w1.canvas();
                // Only draw if from_x is within bounds to avoid clipping issues
                if from_x < screen_width as i32 {
                    composed.draw_from(&canvas1, (from_x.max(0)) as u32, 0);
                }
            }

            if let Some(w2) = self.window_for_handle_mut(to_handle) {
                let canvas2 = w2.canvas();
                // Only draw if to_x is within bounds
                if to_x < screen_width as i32 {
                    composed.draw_from(&canvas2, (to_x.max(0)) as u32, 0);
                }
            }

            self.composited_id = Some(id);
            if let Some(display) = self.display {
                let mut disp = display.lock().await;
                disp.draw(get_buffer_slice(id)).await;
            }

            release_buffer(id);
            Timer::after(Duration::from_millis(ANIM_FRAME_DELAY_MS)).await;
        }

        self.current_index = to_index;
    }

    pub async fn composite(&mut self) -> Option<&'static [u8]> {
        let mut fb = allocate_buffer().await?;
        let id = fb.id();
        let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
        composed.clear();

        match self.view_mode {
            ViewMode::Single => {
                if let Some(window) = self.windows.get_mut(self.current_index) {
                    let canvas = window.canvas();
                    composed.draw_from(&canvas, 0, 0);
                }
            }
            ViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.windows.len();

                if let Some(w1) = self.windows.get_mut(i1) {
                    let canvas1 = w1.canvas();
                    composed.draw_from(&canvas1, 0, 0);
                }

                if let Some(w2) = self.windows.get_mut(i2) {
                    let canvas2 = w2.canvas();
                    composed.draw_from(&canvas2, 64, 0); // right half
                }
            }
        }

        self.composited_id = Some(id);
        Some(get_buffer_slice(id))
    }

    pub fn release_last(&mut self) {
        if let Some(id) = self.composited_id.take() {
            release_buffer(id);
        }
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    pub fn is_focused(&self, handle: WindowHandle) -> bool {
        self.windows
            .get(self.current_index)
            .map(|w| w.handle() == handle)
            .unwrap_or(false)
    }

    pub fn poll_input(
        &mut self,
        handle: WindowHandle,
    ) -> Option<crate::system::services::human_input_srv::HumanInputEvent> {
        self.window_for_handle_mut(handle)
            .and_then(|w| w.input_receiver().try_receive().ok())
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