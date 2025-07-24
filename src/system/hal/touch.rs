use alloc::boxed::Box;
use async_trait::async_trait;
use embedded_hal::spi::{Error, ErrorType};

/// Trait for async touch input
#[async_trait(?Send)]
pub trait AsyncTouch {
    async fn read_xyz(&mut self) -> (u16, u16, u16);
}