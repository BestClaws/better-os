// HTTP client errors

use crate::libs::hps::error::HpsError;
use defmt::Format;

/// HTTP client error type
#[derive(Debug, Clone, Copy, PartialEq, Format)]
pub enum Error {
    /// URL is invalid or too long
    InvalidUrl,
    /// Request body is too large
    RequestTooLarge,
    /// Response parsing failed
    ResponseError,
    /// Service not ready or connection failed
    ServiceUnavailable,
    /// Request timeout
    Timeout,
    /// HPS service error
    HpsError(HpsError),
    /// JSON parsing error
    JsonError,
}

impl From<HpsError> for Error {
    fn from(err: HpsError) -> Self {
        Error::HpsError(err)
    }
}

/// HTTP client result type
pub type Result<T> = core::result::Result<T, Error>;
