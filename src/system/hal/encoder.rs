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

// Trait for state handling (object-safe, no async methods)
pub trait EncoderStateHandler {
    // Synchronous method to get current state based on pin readings
    fn get_state(&mut self) -> Result<EncoderState, EncoderError>;
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

    // Async method for waiting and computing state (not part of trait)
    pub async fn wait_for_next_state(&mut self) -> Result<EncoderState, EncoderError> {
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
        self.get_state()
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
    fn get_state(&mut self) -> Result<EncoderState, EncoderError> {
        // Read pin states (per TODO: no unwrap)
        let a_high = self.a_pin.is_high().map_err(|_| EncoderError::PinError)?;
        let b_high = self.b_pin.is_high().map_err(|_| EncoderError::PinError)?;

        // Determine state (per TODO: no panic)
        match (a_high, b_high) {
            (false, true) => Ok(EncoderState::Ccw(1.0)),  // Placeholder radians/sec
            (false, false) => Ok(EncoderState::Cw(1.0)), // Placeholder radians/sec
            _ => Err(EncoderError::InvalidState),
        }
    }
}

// Dynamic dispatch example (synchronous)
fn use_encoder(encoder: &mut dyn EncoderStateHandler) -> Result<EncoderState, EncoderError> {
    encoder.get_state()
}

// Async wrapper for dynamic dispatch
async fn use_encoder_async<P: InputPin + Wait>(
    encoder: &mut EncoderDriver<P>,
) -> Result<EncoderState, EncoderError> {
    encoder.wait_for_next_state().await
}