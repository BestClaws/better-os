use alloc::boxed::Box;
       // v1.0.0
      // v1.0.0
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
}

#[async_trait(?Send)]
pub trait AsyncEncoder {
    async fn next(&mut self) -> Result<EncoderState, EncoderError>;
}

