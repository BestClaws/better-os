use alloc::boxed::Box;
use async_trait::async_trait;
use embedded_graphics::image::Image;
use embedded_graphics::pixelcolor::BinaryColor;
use tinybmp::Bmp;
use crate::system::services::human_input::HumanInputEvent;

#[async_trait(?Send)]
pub(crate) trait AsyncDisplay {
    async fn init(&mut self);
    async fn draw(&mut self, event: HumanInputEvent);
}