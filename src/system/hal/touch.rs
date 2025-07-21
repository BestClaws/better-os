use alloc::boxed::Box;
use async_trait::async_trait;
use embedded_hal::spi::{Error, ErrorType};

#[async_trait(?Send)]
pub(crate) trait AsyncTouch {
    async fn read_xy(&mut self) -> (u16, u16, u16);

}