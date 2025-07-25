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

// Animation tuning globals
const ANIM_STEPS: usize = 8;
const ANIM_FRAME_DELAY_MS: u64 = 2;

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

/// Trait to handle pixel copying for different pixel color types.
trait BlitPixel: PixelColor {
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
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            current_window: 0,
            view_mode: ViewMode::Single,
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
            let mut working_buff = [0u8; FRAME_BUFFER_SIZE];
            let mut dest_canvas = Canvas::<Gray4>::new(&mut working_buff, FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);
            self.composite(&mut dest_canvas).await;
            let mut disp = display.lock().await;
            let then = Instant::now();
            disp.draw(dest_canvas.buffer_mut(), FRAME_SCALE_FACTOR).await;
            info!("frame time: {}", (Instant::now() - then).as_millis());
        }

        self.redraw_requests.clear();
    }

    /// Return the handle of the currently focused window
    pub fn current_handle(&self) -> Option<WindowHandle> {
        if self.windows.len() > 0 {
            return Some(self.windows[self.current_window].handle());
        }
        None
    }

    /// Find a mutable reference to a window by its handle (for external callers)
    pub fn get_window_mut(&mut self, handle: WindowHandle) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.handle() == handle)
    }

    pub async fn alloc_window(
        &mut self,
        width: u32,
        height: u32,
        id: usize,
    ) -> Option<WindowHandle> {
        let window = Window::new(width, height, id).await;
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
            self.current_window = (self.current_window + 1) % self.windows.len();
        }
    }

    pub fn prev_window(&mut self) {
        if !self.windows.is_empty() {
            self.current_window = (self.current_window + self.windows.len() - 1) % self.windows.len();
        }
    }

    pub async fn animate_slide(&mut self, dir: SlideDir) {
        if self.windows.len() < 2 {
            return;
        }

        let from_index = self.current_window;
        let to_index = match dir {
            SlideDir::Left => {
                if self.current_window == 0 {
                    self.windows.len() - 1
                } else {
                    self.current_window - 1
                }
            }
            SlideDir::Right => (self.current_window + 1) % self.windows.len(),
        };

        let mut composed_buf = [0u8; FRAME_BUFFER_SIZE];
        let mut dest_canvas: Canvas<Gray4> = Canvas::<Gray4>::new(&mut composed_buf, FRAME_BUFFER_WIDTH, FRAME_BUFFER_HEIGHT);

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

            // Borrow windows sequentially
            let src1_canvas = {
                let src1_win = &mut self.windows[from_index];
                src1_win.canvas().await
            };
            let src2_canvas = {
                let src2_win = &mut self.windows[to_index];
                src2_win.canvas().await
            };
            blit(&mut dest_canvas, &src1_canvas, from_x, 0);
            blit(&mut dest_canvas, &src2_canvas, to_x, 0);

            if let Some(display) = self.display {
                let mut disp = display.lock().await;
                disp.draw(dest_canvas.buffer_mut(), FRAME_SCALE_FACTOR).await;
            }

            Timer::after(Duration::from_millis(ANIM_FRAME_DELAY_MS)).await;
        }

        self.current_window = to_index;
    }

    pub async fn composite<'a>(&'a mut self, working_buff: &mut Canvas<'a, Gray4>) {
        working_buff.clear(Gray4::BLACK).unwrap();

        match self.view_mode {
            ViewMode::Single => {
                let src_canvas = self.windows[self.current_window].canvas().await;
                blit(working_buff, &src_canvas, 0, 0);
            }

            ViewMode::Split => {
                let i1 = self.current_window;
                let i2 = (self.current_window + 1) % self.windows.len();

                // Borrow windows sequentially
                let src1_canvas = {
                    let src1_win = &mut self.windows[i1];
                    src1_win.canvas().await
                };
                let src2_canvas = {
                    let src2_win = &mut self.windows[i2];
                    src2_win.canvas().await
                };
                blit(working_buff, &src1_canvas, 0, 0);
                blit(working_buff, &src2_canvas, (FRAME_BUFFER_WIDTH / 2) as i32, 0);
            }
        }
    }

    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Check if a window handle is the currently focused window
    pub fn is_focused(&self, handle: WindowHandle) -> bool {
        self.windows[self.current_window].handle() == handle
    }

    /// Poll input event from a window's input receiver (if available)
    pub fn poll_input(
        &mut self,
        handle: WindowHandle,
    ) -> Option<crate::system::services::human_input_srv::HumanInputEvent> {
        self.get_window_mut(handle)
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