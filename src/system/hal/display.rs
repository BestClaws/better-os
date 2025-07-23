use alloc::boxed::Box;
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait AsyncDisplay {
    async fn init(&mut self);
    async fn draw(&mut self, buffer: &[u8], scale: u32);
    async fn clear(&mut self, color: u16);
    async fn set_orientation(&mut self, orientation: Orientation);
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
}

#[derive(Clone, Copy)]
pub enum Orientation {
    Portrait,
    PortraitFlipped,
    Landscape,
    LandscapeFlipped,
}