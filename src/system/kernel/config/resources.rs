use crate::system::resources::framebuffer::FrameBufferPool;

// frame buffer resource configuration
pub const FRAME_BUFFER_WIDTH: u32 = 200;
pub const FRAME_BUFFER_HEIGHT: u32 = 150;

pub const FRAME_SCALE_FACTOR: u32 =1;

pub const FRAME_BUFFER_BIT_DEPTH: usize = 4; // memory consumed by each pixel
pub const FRAME_BUFFER_SIZE: usize = ((FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT) as f32 * (FRAME_BUFFER_BIT_DEPTH as f32 / 8.0)) as usize;
pub const FRAME_BUFFER_COUNT: usize = 3;

pub const MAX_BATCH_LINES: usize = 1;


