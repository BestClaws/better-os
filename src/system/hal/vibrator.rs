use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::Duration;

#[async_trait(?Send)]
pub trait AsyncVibrator {
    async fn vibrate(&mut self, duration: Duration);
}

