use crate::system::resources::framebuffer::FrameBufferPool;

// frame buffer resource configuration
pub const FRAME_BUFFER_WIDTH: u32 = 80;
pub const FRAME_BUFFER_HEIGHT: u32 = 60;

pub const FRAME_SCALE_FACTOR: u32 =4;

pub const FRAME_BUFFER_BIT_DEPTH: usize = 16; // memory consumed by each pixel
pub const FRAME_BUFFER_SIZE: usize = ((FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT) as f32 * (FRAME_BUFFER_BIT_DEPTH as f32 / 8.0)) as usize;
pub const FRAME_BUFFER_COUNT: usize = 3;


