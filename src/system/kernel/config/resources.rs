use crate::system::resources::framebuffer::FrameBufferPool;

// frame buffer resource configuration
pub const FRAME_BUFFER_WIDTH: usize = 120;
pub const FRAME_BUFFER_HEIGHT: usize = 160;

pub const FRAME_BUFFER_BIT_DEPTH: usize = 8; // memory consumed by each pixel
pub const FRAME_BUFFER_SIZE: usize = ((FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT) as f32 * (FRAME_BUFFER_BIT_DEPTH as f32 / 8.0)) as usize;
pub const FRAME_BUFFER_COUNT: usize = 4;


