use alloc::boxed::Box;
use async_trait::async_trait;
use crate::system::services::human_input::HumanInputEvent;

#[async_trait(?Send)]
pub(crate) trait AsyncDisplay {
    async fn init(&mut self);
    async fn draw(&mut self, buffer: &[u8]);

}