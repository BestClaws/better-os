use alloc::boxed::Box;
use embedded_hal::digital::InputPin;       // v1.0.0
use embedded_hal_async::digital::Wait;      // v1.0.0
use embassy_time::{Duration, Timer};
use async_trait::async_trait;

// --- your driver and traits ---

#[derive(Debug)]
pub enum EncoderState {
    // Cw(f32),
    // Ccw(f32),
    Cw,
    Ccw
}

#[derive(Debug)]
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
        if embassy_time::with_timeout(
            Duration::from_millis(100000),
            self.a_pin.wait_for_falling_edge(),
        )
            .await
            .is_err()
        {
            return Err(EncoderError::Timeout);
        }

        // 2) debounce
        Timer::after(Duration::from_millis(1)).await;

        // 3) sample
        let a = self.a_pin.is_high().map_err(|_| EncoderError::PinError)?;
        let b = self.b_pin.is_high().map_err(|_| EncoderError::PinError)?;

        // 4) decode
        match (a, b) {
            (false, true)  => Ok(EncoderState::Ccw),
            (false, false) => Ok(EncoderState::Cw),
            _              => Err(EncoderError::InvalidState),
        }
    }
}
