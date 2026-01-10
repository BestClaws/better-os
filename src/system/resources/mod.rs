pub(crate) mod framebuffer;
pub(crate) mod input_channels;

/// HPS (HTTP Proxy Service) Buffer Configuration
/// 
/// These values define the maximum buffer sizes for HTTP operations over BLE.
/// Total budget: 10KB (1KB URI + 4KB headers + 5KB body)
/// 
/// Adjust these values in one place to sync across:
/// - Rust codebase (types, service implementation)
/// - Python HPS server (extras/tools/hps_server.py)
pub const HPS_MAX_URI_SIZE: usize = 1024;      // 1KB for URLs
pub const HPS_MAX_HEADERS_SIZE: usize = 4096;  // 4KB for request + response headers
pub const HPS_MAX_BODY_SIZE: usize = 5120;     // 5KB for request + response body
