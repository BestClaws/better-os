use alloc::boxed::Box;
use embedded_hal::digital::InputPin; // v1.0.0
use embedded_hal_async::digital::Wait; // v1.0.0
use embassy_time::{Duration, Timer};

// Encoder state with radians/sec (per TODO)
#[derive(Debug)]
pub enum EncoderState {
    Cw(f32),  // Clockwise, radians/sec
    Ccw(f32), // Counter-clockwise, radians/sec
}

// Custom error type
#[derive(Debug)]
pub enum EncoderError {
    PinError,     // Error reading pin state
    InvalidState, // Invalid encoder state
    Timeout,      // Timeout waiting for state change
}

// Object-safe trait for synchronous pin operations
pub trait EncoderPin: InputPin {}

// Implement EncoderPin for types that implement InputPin
impl<T: InputPin> EncoderPin for T {}

// Trait for pin access (object-safe, generic error type)
pub trait EncoderPins {
    type Error: embedded_hal::digital::Error;

    fn input_a(&mut self) -> &mut dyn EncoderPin<Error = Self::Error>;
    fn input_b(&mut self) -> &mut dyn EncoderPin<Error = Self::Error>;
}

#[async_trait::async_trait]
// Trait for state handling (object-safe, no async methods)
pub trait EncoderStateHandler {
    // Synchronous method to get current state based on pin readings
    async fn wait_for_next_state(&mut self) -> Result<EncoderState, EncoderError>;
}

// Encoder driver struct (generic over pin type)
pub struct EncoderDriver<P: InputPin + Wait> {
    a_pin: P,
    b_pin: P,
}

impl<P: InputPin + Wait> EncoderDriver<P> {
    pub fn init(a_pin: P, b_pin: P) -> Self {
        Self { a_pin, b_pin }
    }

}

// Implement EncoderPins for EncoderDriver
impl<P: InputPin + Wait> EncoderPins for EncoderDriver<P> {
    type Error = P::Error;

    fn input_a(&mut self) -> &mut dyn EncoderPin<Error = Self::Error> {
        &mut self.a_pin
    }

    fn input_b(&mut self) -> &mut dyn EncoderPin<Error = Self::Error> {
        &mut self.b_pin
    }
}

// Implement EncoderStateHandler for EncoderDriver
impl<P: InputPin + Wait> EncoderStateHandler for EncoderDriver<P> {


    async fn wait_for_next_state(&mut self) -> Result<EncoderState, EncoderError>{
        // Async method for waiting and computing state (not part of trait)

        // Wait for falling edge on input A with timeout (per TODO: no unwrap)
        if let Err(_) = embassy_time::with_timeout(
            Duration::from_millis(100),
            self.a_pin.wait_for_falling_edge(),
        )
            .await
        {
            return Err(EncoderError::Timeout);
        }

        // Debounce (per TODO: part of driver)
        Timer::after_millis(1).await;

        // Use get_state for pin readings
        Ok(EncoderState::Ccw(1f32))
    }
}



// Async wrapper for dynamic dispatch
async fn use_encoder_async<P: InputPin + Wait>(
    encoder: &mut EncoderDriver<P>,
) -> Result<EncoderState, EncoderError> {
    encoder.wait_for_next_state().await
}