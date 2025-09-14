use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::Format;
use crate::libs::gfx::two_d::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Format)]
pub enum PixelFormat {
    Rgb565,
    Rgb888,
    Rgb666,
    Gray8,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb565 => 2,
            PixelFormat::Rgb888 => 3,
            PixelFormat::Rgb666 => 3,
            PixelFormat::Gray8 => 1,
        }
    }
}

#[async_trait(?Send)]
pub trait AsyncDisplay {
    /// Initialize the display
    async fn init(&mut self);

    /// Paint the entire screen with a single color
    async fn paint_screen(&mut self, color: u8);

    /// Set display brightness
    async fn set_brightness(&mut self, value: u8);


    async fn draw(&mut self, buffer: &[u8], scale: u32);
    async fn draw_region(&mut self, buffer: &[u8], region: Rect, scale: u32);
    async fn set_orientation(&mut self, orientation: Orientation);
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;

    /// Report native pixel format of the driver output buffer
    fn native_pixel_format(&self) -> PixelFormat { PixelFormat::Rgb565 }


}

#[derive(Clone, Copy, Format, PartialEq)]
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}