use crate::system::hal::display::PixelFormat;
use crate::system::input::types::HighLevelEvent;
use crate::system::resources::framebuffer::{FrameBufferHandle, FRAMEBUFFER_POOL};
use crate::system::resources::input_channels::CHANNEL_CAPACITY;
use crate::system::resources::input_channels::{InputChannelHandle, INPUT_CHANNEL_POOL};
use crate::system::ui::drawing_surface::DrawingSurface;
use defmt::{warn, Format};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Receiver, Sender};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Format)]
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
    surface: Option<DrawingSurface<'static>>,
}

impl Window {
    /// Create a new window with the given logical dimensions and pixel format.
    pub async fn new(width: u32, height: u32, id: usize, format: PixelFormat) -> Self {
        Self {
            fb: None,
            input_channel: None,
            width,
            height,
            id,
            surface: Some(DrawingSurface::new_unattached(width, height, format)),
        }
    }

    /// Attach framebuffer and input channel resources.
    pub async fn set_resources(&mut self, fb: FrameBufferHandle, ic: InputChannelHandle) {
        if let Some(surface) = self.surface.as_mut() {
            surface.attach_buffer(FRAMEBUFFER_POOL.get_mut(&fb));
        }
        self.fb = Some(fb);
        self.input_channel = Some(ic);
    }

    /// Update the logical dimensions and pixel format of the window surface.
    pub fn update_surface(&mut self, width: u32, height: u32, format: PixelFormat) {
        self.width = width;
        self.height = height;
        if let Some(surface) = self.surface.as_mut() {
            surface.reconfigure(width, height, format);
        }
    }

    /// Return a mutable reference to the drawing surface slot.
    pub fn surface(&mut self) -> &mut Option<DrawingSurface<'static>> {
        &mut self.surface
    }

    /// Return this window’s handle.
    pub fn handle(&self) -> WindowHandle {
        WindowHandle { id: self.id }
    }

    /// Get ID of the framebuffer (for compositing).
    pub fn framebuffer_id(&self) -> Option<&FrameBufferHandle> {
        self.fb.as_ref()
    }

    /// Return this window’s logical width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Return this window’s logical height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Return reference to the input channel sender.
    pub async fn input_sender(
        &mut self,
    ) -> Option<Sender<CriticalSectionRawMutex, HighLevelEvent, CHANNEL_CAPACITY>> {
        self.input_channel
            .as_ref()
            .map(|channel| INPUT_CHANNEL_POOL.sender(channel))
    }

    /// Return reference to the input channel receiver.
    pub fn input_receiver(
        &self,
    ) -> Option<Receiver<CriticalSectionRawMutex, HighLevelEvent, CHANNEL_CAPACITY>> {
        self.input_channel
            .as_ref()
            .map(|channel| INPUT_CHANNEL_POOL.receiver(channel))
    }

    /// Release held framebuffer and input channel.
    pub fn relax(&mut self) {
        if let Some(surface) = self.surface.as_mut() {
            surface.detach_buffer();
        }
        if let Some(fb_handle) = self.fb.take() {
            FRAMEBUFFER_POOL.release(&fb_handle);
        } else {
            warn!("Window {:?} released without framebuffer", self.handle());
        }

        if let Some(channel_handle) = self.input_channel.take() {
            INPUT_CHANNEL_POOL.release(&channel_handle);
        } else {
            warn!("Window {:?} released without input channel", self.handle());
        }
    }
}
