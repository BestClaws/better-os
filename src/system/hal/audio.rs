use defmt::Format;
use esp_hal::i2s::master::Error as I2sError;

#[derive(Debug, Format)]
pub enum AudioError {
    AlreadyRunning,
    I2s(I2sError),
}

impl From<I2sError> for AudioError {
    fn from(value: I2sError) -> Self {
        Self::I2s(value)
    }
}

pub trait AsyncAudioSink {
    fn play_loop(&mut self, data: &'static [i16]) -> Result<(), AudioError>;
}
