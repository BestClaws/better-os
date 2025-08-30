use crate::system::resources::framebuffer::FrameBufferPool;

// frame buffer resource configuration


pub const DISPLAY_WIDTH: u32 = 466;
pub const DISPLAY_HEIGHT: u32 = 466;


pub const FRAME_SCALE_FACTOR: u32 =2;
pub const FRAME_BUFFER_WIDTH: u32 = DISPLAY_WIDTH / FRAME_SCALE_FACTOR;
pub const FRAME_BUFFER_HEIGHT: u32 = DISPLAY_HEIGHT / FRAME_SCALE_FACTOR;



pub const FRAME_BUFFER_BIT_DEPTH: usize = 4; // memory consumed by each pixel
pub const FRAME_BUFFER_SIZE: usize = ((FRAME_BUFFER_WIDTH * FRAME_BUFFER_HEIGHT) as f32 * (FRAME_BUFFER_BIT_DEPTH as f32 / 8.0)) as usize;
pub const FRAME_BUFFER_COUNT: usize = 3;



/// Configuration: Maximum allowed geometry size for models
pub(crate) const MAX_TRIANGLES: usize = 740;
pub(crate) const MAX_VERTICES: usize = 200;