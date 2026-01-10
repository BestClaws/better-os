// HPS (HTTP Proxy Service) Client Implementation
// Based on Bluetooth SIG HPS v1.0 specification
//
// Architecture:
// - libs/hps: Protocol types and errors (this module)
// - services/hps_service: BLE connection and GATT operations
// - services/http_service: HTTP abstraction layer
// - libs/http: User-facing HTTP client API (reqwest-like)

pub mod error;
pub mod types;

// Legacy modules (no longer used in new architecture)
pub mod client;  // Old channel-based client (deprecated)
pub mod gatt;    // GATT stubs (operations moved to hps_service)

pub use error::HpsError;
pub use types::{
    DataStatus, HttpMethod, HttpRequest, HttpResponse, HttpStatusCode, HpsCharacteristics,
    HpsUuids,
};
