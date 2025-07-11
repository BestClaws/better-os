#![allow(unused)]

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use crate::system::services::human_input_srv::HumanInputEvent;
use crate::system::ui::canvas::Canvas;
use crate::system::ui::framebuffer::FrameBufferHandle;
use crate::system::ui::input_channels::InputChannelHandle;
use crate::system::ui::input_channels::CHANNEL_CAPACITY;

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
    pub fn new(
        fb: FrameBufferHandle,
        input_channel: InputChannelHandle,
        width: usize,
        height: usize,
        id: usize,
    ) -> Self {
        Self {
            fb,
            input_channel,
            width: width as u32,
            height: height as u32,
            id,
        }
    }

    /// Returns a fresh Canvas that draws on this window's framebuffer.
    pub fn canvas(&mut self) -> Canvas {
        Canvas::new(self.fb.buffer_mut(), self.width, self.height)
    }

    /// Return this window’s handle.
    pub fn handle(&self) -> WindowHandle {
        WindowHandle { id: self.id }
    }

    /// Get ID of the framebuffer (for compositing).
    pub fn framebuffer_id(&self) -> usize {
        self.fb.id()
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
    pub fn input_sender(&self) -> &embassy_sync::channel::Sender<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        self.input_channel.sender()
    }

    /// Return reference to the input channel receiver.
    pub fn input_receiver(&self) -> &embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, HumanInputEvent, CHANNEL_CAPACITY> {
        self.input_channel.receiver()
    }
}

// Possibly implement Drop if you want to release input channel on window drop
