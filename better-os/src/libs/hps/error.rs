use defmt::Format;

/// HPS-specific errors
#[derive(Debug, Clone, Copy, Format, PartialEq, Eq)]
pub enum HpsError {
    /// The URI, HTTP Headers, or HTTP Entity Body characteristics are not set correctly
    InvalidRequest,

    /// Network connection is not available
    NetworkNotAvailable,

    /// Insufficient resources to originate HTTP request
    InsufficientResources,

    /// Client Characteristic Configuration Descriptor not configured for notifications
    NotificationsNotConfigured,

    /// An HTTP request is already in progress
    ProcedureAlreadyInProgress,

    /// BLE/GATT communication error
    BleError,

    /// Service or characteristic not found
    NotFound,

    /// Response data too large (truncated)
    DataTruncated,

    /// Timeout waiting for response
    Timeout,

    /// Invalid state for operation
    InvalidState,

    /// Buffer too small
    BufferTooSmall,

    /// UTF-8 encoding error
    EncodingError,
}

impl From<HpsError> for u8 {
    fn from(error: HpsError) -> u8 {
        match error {
            HpsError::InvalidRequest => 0x81,
            HpsError::NetworkNotAvailable => 0x82,
            _ => 0x00,
        }
    }
}

impl HpsError {
    pub fn from_att_error(code: u8) -> Option<Self> {
        match code {
            0x81 => Some(HpsError::InvalidRequest),
            0x82 => Some(HpsError::NetworkNotAvailable),
            _ => None,
        }
    }
}
