use alloc::boxed::Box;
use embedded_hal::digital::InputPin;       // v1.0.0
use embedded_hal_async::digital::Wait;      // v1.0.0
use embassy_time::{Duration, Timer};
use async_trait::async_trait;
use defmt::Format;
use crate::system::hal::encoder::{EncoderError, EncoderState};

pub struct ButtonDriver<P: InputPin + Wait> {
    pin: P,
}

impl<P: InputPin + Wait> ButtonDriver<P> {
    pub fn new(pin: P) -> Self {
        Self { pin }
    }
}

#[async_trait(?Send)]
pub trait AsyncButton {
    async fn wait_for_press(&mut self);
    async fn wait_for_release(&mut self);
}

#[async_trait(?Send)]
impl<P: InputPin + Wait> AsyncButton for ButtonDriver<P> {
    async fn wait_for_press(&mut self)  {
            self.pin.wait_for_falling_edge().await.expect("TODO: panic message");
            Timer::after(Duration::from_millis(2)).await;

        }


    async fn wait_for_release(&mut self)  {
        self.pin.wait_for_falling_edge().await.expect("TODO: panic message");
        Timer::after(Duration::from_millis(2)).await;
    }
}
