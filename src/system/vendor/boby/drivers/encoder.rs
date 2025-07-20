use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_time::{Duration, Timer};
use embedded_hal::digital::InputPin;
use embedded_hal_async::digital::Wait;
use crate::system::hal::encoder::{AsyncEncoder, EncoderError, EncoderState};

pub struct EncoderDriver<P: InputPin + Wait> {
    a_pin: P,
    b_pin: P,
}

impl<P: InputPin + Wait> EncoderDriver<P> {
    pub fn new(a: P, b: P) -> Self {
        Self { a_pin: a, b_pin: b }
    }
}


#[async_trait(?Send)]
impl<P: InputPin + Wait> AsyncEncoder for EncoderDriver<P> {
    async fn next(&mut self) -> Result<EncoderState, EncoderError> {

        self.a_pin.wait_for_any_edge().await.unwrap();

        // 2) debounce
        Timer::after(Duration::from_millis(2)).await;

        // 3) sample
        let a = self.a_pin.is_high().map_err(|_| EncoderError::PinError)?;
        let b = self.b_pin.is_high().map_err(|_| EncoderError::PinError)?;

 
        // 4) decode
        return match (a, b) {
            (false, true)  => Ok(EncoderState::Ccw),
            (false, false) => Ok(EncoderState::Cw),
            (true, false) => Ok(EncoderState::Ccw),
            (true, true)  => Ok(EncoderState::Cw),
        }

    }
}
