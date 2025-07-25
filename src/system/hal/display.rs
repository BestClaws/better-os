use alloc::boxed::Box;
use alloc::vec;
use async_trait::async_trait;
use defmt::info;
use display_interface::DataFormat;
use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};

#[async_trait(?Send)]
pub trait AsyncDisplay {
    async fn init(&mut self);
    async fn draw_gray4(&mut self, buffer: &[u8], scale: u32);
    async fn
    draw(&mut self, buffer: &[u8], scale: u32);
    async fn clear(&mut self, color: u16);
    async fn set_orientation(&mut self, orientation: Orientation);
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
}

#[derive(Clone, Copy)]
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}