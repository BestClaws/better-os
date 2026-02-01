pub(crate) mod framebuffer;


pub const HPS_MAX_URI_SIZE: usize = 1024; // 1KB for URLs
pub const HPS_MAX_HEADERS_SIZE: usize = 4096; // 4KB for request + response headers
pub const HPS_MAX_BODY_SIZE: usize = 5120; // 5KB for request + response body
