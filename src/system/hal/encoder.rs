use alloc::boxed::Box;
use embedded_hal::digital::InputPin;       // v1.0.0
use embedded_hal_async::digital::Wait;      // v1.0.0
use embassy_time::{Duration, Timer};
use async_trait::async_trait;
use defmt::Format;
// --- your driver and traits ---

#[derive(Debug, Format)]
pub enum EncoderState {
    // Cw(f32),
    // Ccw(f32),
    Cw,
    Ccw
}

#[derive(Debug, Format)]
pub enum EncoderError {
    PinError,
    InvalidState,
    Timeout,
}

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
pub trait AsyncEncoder {
    async fn next(&mut self) -> Result<EncoderState, EncoderError>;
}

#[async_trait(?Send)]
impl<P: InputPin + Wait> AsyncEncoder for EncoderDriver<P> {
    async fn next(&mut self) -> Result<EncoderState, EncoderError> {

        loop {
            self.a_pin.wait_for_falling_edge().await;



            // 2) debounce
            Timer::after(Duration::from_millis(2)).await;

            // 3) sample
            let a = self.a_pin.is_high().map_err(|_| EncoderError::PinError)?;
            let b = self.b_pin.is_high().map_err(|_| EncoderError::PinError)?;

            // 4) decode
            return match (a, b) {
                (false, true)  => Ok(EncoderState::Ccw),
                (false, false) => Ok(EncoderState::Cw),
                _              => continue,
            }
        }

    }
}
