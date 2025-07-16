
// frame buffer resource configuration
pub const FRAME_BUFFER_WIDTH: usize = 128;
pub const FRAME_BUFFER_HEIGHT: usize = 64;

pub const FRAME_BUFFER_BIT_DEPTH: usize = 1; // memory consumed by each pixel
pub const FRAME_BUFFER_SIZE: usize = FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT * (FRAME_BUFFER_BIT_DEPTH / 8);
pub const FRAME_BUFFER_COUNT: usize = 8;