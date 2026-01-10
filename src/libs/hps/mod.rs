// HPS (HTTP Proxy Service) Client Implementation
// Based on Bluetooth SIG HPS v1.0 specification

pub mod client;
pub mod error;
pub mod gatt;
pub mod types;

pub use client::HpsClient;
pub use error::HpsError;
pub use gatt::HpsGattConnector;
pub use types::{
    DataStatus, HttpMethod, HttpRequest, HttpResponse, HttpStatusCode, HpsCharacteristics,
    HpsUuids,
};
