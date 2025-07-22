use alloc::boxed::Box;
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait AsyncDisplay {
    async fn init(&mut self);
    async fn draw(&mut self, buffer: &[u8]);
    async fn clear(&mut self, color: u16);
    async fn set_orientation(&mut self, orientation: crate::system::vendor::boby::drivers::ili9341::driver::Orientation);
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}