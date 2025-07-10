use crate::system::hal::display::AsyncDisplay;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::{allocate_buffer, get_buffer_slice, release_buffer};
use crate::system::ui::window::{Window, WindowHandle};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use alloc::boxed::Box;
use heapless::Vec;
use embassy_time::{Timer, Duration};

#[derive(Clone, Copy, Debug)]
pub enum ViewMode {
    Single,
    Split,
}

pub struct UICompositor {
    windows: Vec<Window, 8>,
    current_index: usize,
    view_mode: ViewMode,
    composited_id: Option<usize>,
    next_id: usize,

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
            next_id: 0,

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
        if !self.redraw_requests.iter().any(|h| *h == handle) {
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

    pub async fn alloc_window_with_canvas(
        &mut self,
        width: usize,
        height: usize,
        id: usize,
    ) -> Option<(WindowHandle, Canvas)> {
        let fb = allocate_buffer().await?;
        let window = Window::new(fb, width, height, id);
        let handle = window.handle();

        self.windows.push(window).ok()?;
        let window = self.windows.iter_mut().find(|w| w.handle() == handle)?;
        let canvas = window.canvas();

        Some((handle, canvas))
    }

    pub async fn alloc_window(&mut self, width: usize, height: usize) -> Option<WindowHandle> {
        let fb = allocate_buffer().await?;
        let id = self.next_id;
        self.next_id += 1;

        let window = Window::new(fb, width, height, id);
        let handle = window.handle();

        self.windows.push(window).ok()?;
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

    pub async fn animate_slide(&mut self, dir: SlideDir) {
        if let Some((from, to)) = self.last_transition_handles(dir) {
            for offset in (0..=128).step_by(8) {
                let offset = match dir {
                    SlideDir::Left => offset,
                    SlideDir::Right => 128 - offset,
                };

                if let Some(frame) = self.composite_slide(from, to, offset).await {
                    if let Some(display) = self.display {
                        let mut disp = display.lock().await;
                        disp.draw(frame).await;
                    }
                    Timer::after(Duration::from_millis(10)).await;
                    self.release_last();
                }
            }
        }
    }

    pub async fn composite(&mut self) -> Option<&'static [u8]> {
        let mut fb = allocate_buffer().await?;
        let id = fb.id();
        let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
        composed.clear();

        match self.view_mode {
            ViewMode::Single => {
                if let Some(window) = self.windows.get_mut(self.current_index) {
                    let mut canvas = window.canvas();
                    composed.draw_from(&canvas, 0, 0);
                }
            }
            ViewMode::Split => {
                let i1 = self.current_index;
                let i2 = (self.current_index + 1) % self.windows.len();

                if let Some(w1) = self.windows.get_mut(i1) {
                    let mut canvas1 = w1.canvas();
                    composed.draw_from(&canvas1, 0, 0);
                }

                if let Some(w2) = self.windows.get_mut(i2) {
                    let mut canvas2 = w2.canvas();
                    composed.draw_from(&canvas2, 0, 32);
                }
            }
        }

        self.composited_id = Some(id);
        Some(get_buffer_slice(id))
    }

    pub async fn composite_slide(
        &mut self,
        from: WindowHandle,
        to: WindowHandle,
        offset: i32,
    ) -> Option<&'static [u8]> {
        let mut fb = allocate_buffer().await?;
        let id = fb.id();
        let mut composed = Canvas::new(fb.buffer_mut(), 128, 64);
        composed.clear();

        if let Some(w1) = self.windows.iter_mut().find(|w| w.handle() == from) {
            let mut canvas1 = w1.canvas();
            let x1 = offset.saturating_neg() as u32;
            composed.draw_from(&canvas1, x1.wrapping_sub(offset as u32), 0);
        }

        if let Some(w2) = self.windows.iter_mut().find(|w| w.handle() == to) {
            let mut canvas2 = w2.canvas();
            let x2 = (128 - offset).max(0) as u32;
            composed.draw_from(&canvas2, x2, 0);
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

    pub fn last_transition_handles(&self, dir: SlideDir) -> Option<(WindowHandle, WindowHandle)> {
        if self.windows.is_empty() {
            return None;
        }

        let from = self.current_index;
        let to = match dir {
            SlideDir::Left => (self.current_index + 1) % self.windows.len(),
            SlideDir::Right => {
                if self.current_index == 0 {
                    self.windows.len() - 1
                } else {
                    self.current_index - 1
                }
            }
        };

        Some((
            self.windows.get(from)?.handle(),
            self.windows.get(to)?.handle(),
        ))
    }
}

#[derive(Clone, Copy)]
pub enum SlideDir {
    Left,
    Right,
}
