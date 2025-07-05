use alloc::boxed::Box;
use async_trait::async_trait;

#[async_trait(?Send)]
pub(crate) trait AsyncDisplay {
    async fn init(&mut self);
}