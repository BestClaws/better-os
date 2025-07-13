use core::fmt::Debug;
use embedded_hal_async::i2c::I2c;

pub enum Error<I> where I: I2c {
    I2cError(I::Error),
    Other,
    Timeout,
    FirmwareUploadVerificationFailed,
}

impl<I> Debug for Error<I> where I: I2c {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::I2cError(e) => write!(f, "I2C error: {:?}", e),
            Error::Other => write!(f, "Other error"),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::FirmwareUploadVerificationFailed => write!(f, "Firmware upload verification failed"),
        }
    }
}
