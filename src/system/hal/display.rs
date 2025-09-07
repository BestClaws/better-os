use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::Format;
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::geometry::{Point as EgPoint, Size as EgSize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayPixelFormat {
    Rgb565,
    Rgb888,
    Rgb666,
    Gray8,
}

impl DisplayPixelFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            DisplayPixelFormat::Rgb565 => 2,
            DisplayPixelFormat::Rgb888 => 3,
            DisplayPixelFormat::Rgb666 => 3,
            DisplayPixelFormat::Gray8 => 1,
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
    async fn draw_region(&mut self, buffer: &[u8], region: Rectangle, scale: u32);
    async fn set_orientation(&mut self, orientation: Orientation);
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;

    /// Report native pixel format of the driver output buffer
    fn native_pixel_format(&self) -> DisplayPixelFormat { DisplayPixelFormat::Rgb565 }


}

#[derive(Clone, Copy, Format, PartialEq)]
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}