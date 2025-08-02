#![allow(unused)]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};
use embedded_graphics::framebuffer::Framebuffer;
use embedded_graphics_core::pixelcolor::Gray4;
use embedded_graphics_core::prelude::PixelColor;
use esp_hal::interrupt::map;
use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::{Canvas, PixelColorExt};
use crate::system::resources::framebuffer::{FrameBufferHandle, FRAMEBUFFER_POOL};
use crate::system::resources::input_channels::{InputChannelHandle, INPUT_CHANNEL_POOL};
use crate::system::resources::input_channels::CHANNEL_CAPACITY;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowHandle {
    id: usize,
}

/// Represents a UI window backed by a framebuffer and owning an input channel.
pub struct Window {
    fb: Option<FrameBufferHandle>,
    input_channel: Option<InputChannelHandle>,
    width: u32,
    height: u32,
    id: usize,
}

impl Window {
    /// Create a new Window.
    pub async fn new(
        width: u32,
        height: u32,
        id: usize,
    ) -> Self {


        Self {
            fb: None,
            input_channel: None,
            width,
            height,
            id,
        }
    }

    pub async fn set_resources(&mut self, fb: FrameBufferHandle, ic: InputChannelHandle) {
        self.fb = Some(fb);
        self.input_channel = Some(ic);
    }

    /// Returns a fresh Canvas that draws on this window's framebuffer.
    pub fn canvas<C: PixelColorExt>(&mut self) -> Option<Canvas<'static, C>> {
        self.fb.as_ref().map(|fb| {
            C::new_canvas(FRAMEBUFFER_POOL.get_mut(fb), self.width, self.height)
        })
        
    }
    /// Return this window’s handle.
    pub fn handle(&self) -> WindowHandle {
        WindowHandle { id: self.id }
    }

    /// Get ID of the framebuffer (for compositing).
    pub fn framebuffer_id(&self) -> Option<&FrameBufferHandle> {
        self.fb.as_ref()
    }

    /// Return this window’s raw width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Return this window’s raw height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Return reference to the input channel sender.
    pub async fn input_sender(&mut self) -> Option<Sender<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>> {
        self.input_channel.as_ref().map(|channel| INPUT_CHANNEL_POOL.sender(channel))
    }

    /// Return reference to the input channel receiver.
    pub fn input_receiver(&self) -> Option<Receiver<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY>> {
        self.input_channel.as_ref().map(|channel| INPUT_CHANNEL_POOL.receiver(channel))
    }

    /// give away held framebuffer and input channel
    pub fn relax(&mut self) {
        assert!(self.fb.is_some());
        assert!(self.input_channel.is_some());

        FRAMEBUFFER_POOL.release(self.fb.as_mut().unwrap());
        INPUT_CHANNEL_POOL.release(self.input_channel.as_ref().unwrap());


    }
}

// Possibly implement Drop if you want to release input channel on window drop
