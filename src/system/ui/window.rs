#![allow(unused)]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embedded_graphics::framebuffer::Framebuffer;
use esp_hal::interrupt::map;
use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::Canvas;
use crate::system::resources::framebuffer::{FrameBufferHandle, FRAMEBUFFER_POOL};
use crate::system::resources::input_channels::{InputChannelHandle, INPUT_CHANNEL_POOL};
use crate::system::resources::input_channels::CHANNEL_CAPACITY;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowHandle {
    id: usize,
}

/// Represents a UI window backed by a framebuffer and owning an input channel.
pub struct Window {
    fb: FrameBufferHandle,
    input_channel: InputChannelHandle,
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
            fb: FRAMEBUFFER_POOL.allocate().await.unwrap(),
            input_channel: INPUT_CHANNEL_POOL.allocate().await.unwrap(),
            width: width as u32,
            height: height as u32,
            id,
        }
    }

    /// Returns a fresh Canvas that draws on this window's framebuffer.
    pub async fn canvas(&mut self) -> Canvas {
       let buf = FRAMEBUFFER_POOL.get_mut(&self.fb);
        Canvas::new(buf, self.width, self.height)
    }

    /// Return this window’s handle.
    pub fn handle(&self) -> WindowHandle {
        WindowHandle { id: self.id }
    }

    /// Get ID of the framebuffer (for compositing).
    pub fn framebuffer_id(&self) -> &FrameBufferHandle {
        &self.fb
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
    pub fn input_sender(&self) -> embassy_sync::channel::Sender<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        INPUT_CHANNEL_POOL.sender(&self.input_channel)
    }

    /// Return reference to the input channel receiver.
    pub fn input_receiver(&self) -> embassy_sync::channel::Receiver<CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        INPUT_CHANNEL_POOL.receiver(&self.input_channel)   
    }
}

// Possibly implement Drop if you want to release input channel on window drop
