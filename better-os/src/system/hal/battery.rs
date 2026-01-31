use alloc::boxed::Box;
// v1.0.0
// v1.0.0
use async_trait::async_trait;

#[async_trait(?Send)]
pub trait AsyncBattery {
    async fn percent(&mut self) -> u8;
}
