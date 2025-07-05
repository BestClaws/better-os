use embedded_hal::digital::InputPin;       // v1.0.0
use embedded_hal_async::digital::Wait;      // v1.0.0
use embassy_time::{Duration, Timer};
use async_trait::async_trait;
use alloc::boxed::Box;
use core::{future::Future, pin::Pin};

// 1) Extension trait: box the HAL’s async fn into a (non-Send) future.
pub trait SendWait: Wait {
    /// Returns a boxed future for waiting on the falling edge.
    fn wait_for_falling_edge_box<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + 'a>>;
}

impl<T> SendWait for T
where
    T: Wait + 'static,
{
    fn wait_for_falling_edge_box<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), T::Error>> + 'a>> {
        // simply box whatever the HAL gave us
        Box::pin(self.wait_for_falling_edge())
    }
}

// ---- your encoder types ----

#[derive(Debug)]
pub enum EncoderState {
    Cw(f32),
    Ccw(f32),
}

#[derive(Debug)]
pub enum EncoderError {
    PinError,
    InvalidState,
    Timeout,
}

pub trait EncoderPin: InputPin {}
impl<T: InputPin> EncoderPin for T {}

pub trait EncoderPins {
    type Error: embedded_hal::digital::Error;
    fn input_a(&mut self) -> &mut dyn EncoderPin<Error = Self::Error>;
    fn input_b(&mut self) -> &mut dyn EncoderPin<Error = Self::Error>;
}

// 2) Allow the handler’s own future to be non-Send
#[async_trait(?Send)]
pub trait AsyncEncoderStateHandler {
    async fn wait_for_next_state(&mut self) -> Result<EncoderState, EncoderError>;
}

pub struct EncoderDriver<P: InputPin + SendWait + 'static> {
    a_pin: P,
    b_pin: P,
}

impl<P: InputPin + SendWait + 'static> EncoderDriver<P> {
    pub fn init(a_pin: P, b_pin: P) -> Self {
        Self { a_pin, b_pin }
    }
}

impl<P: InputPin + SendWait + 'static> EncoderPins for EncoderDriver<P> {
    type Error = P::Error;

    fn input_a(&mut self) -> &mut dyn EncoderPin<Error = Self::Error> {
        &mut self.a_pin
    }

    fn input_b(&mut self) -> &mut dyn EncoderPin<Error = Self::Error> {
        &mut self.b_pin
    }
}

#[async_trait(?Send)]
impl<P: InputPin + SendWait + 'static> AsyncEncoderStateHandler for EncoderDriver<P> {
    async fn wait_for_next_state(&mut self) -> Result<EncoderState, EncoderError> {
        // 1) wait for falling edge on A (no Send bound)
        if embassy_time::with_timeout(
            Duration::from_millis(100000),
            self.a_pin.wait_for_falling_edge_box(),
        )
            .await
            .is_err()
        {
            return Err(EncoderError::Timeout);
        }

        // 2) debounce
        Timer::after(Duration::from_millis(1)).await;

        // 3) sample both pins
        let a_high = self.a_pin.is_high().map_err(|_| EncoderError::PinError)?;
        let b_high = self.b_pin.is_high().map_err(|_| EncoderError::PinError)?;

        // 4) decode quadrature
        match (a_high, b_high) {
            (false, true) => Ok(EncoderState::Ccw(1.0)),
            (false, false) => Ok(EncoderState::Cw(1.0)),
            _ => Err(EncoderError::InvalidState),
        }
    }
}
