use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore};
use portable_atomic::{AtomicU32, Ordering};

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 64;
pub const FRAME_SIZE: usize = WIDTH * HEIGHT / 8;
pub const NUM_BUFFERS: usize = 3;

pub static mut FRAMEBUFFERS: [[u8; FRAME_SIZE]; NUM_BUFFERS] = [[0; FRAME_SIZE]; NUM_BUFFERS];
pub static BUFFER_USED: AtomicU32 = AtomicU32::new(0);
pub static FB_SEMAPHORE: GreedySemaphore<CriticalSectionRawMutex> =
    GreedySemaphore::new(NUM_BUFFERS);

pub struct Framebuffer {
    pub id: usize,
    pub buf: &'static mut [u8; FRAME_SIZE],
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        BUFFER_USED.fetch_and(!(1 << self.id), Ordering::SeqCst);
        FB_SEMAPHORE.release(1);
    }
}

pub fn request_framebuffer() -> Option<Framebuffer> {
    for id in 0..NUM_BUFFERS {
        let mask = 1 << id;
        if BUFFER_USED.fetch_or(mask, Ordering::SeqCst) & mask == 0 {
            unsafe {
                return Some(Framebuffer {
                    id,
                    buf: &mut FRAMEBUFFERS[id],
                });
            }
        }
    }
    None
}


use embassy_sync::channel::Channel;


#[derive(Clone, Copy)]
pub struct SubmitFrame {
    pub id: usize,
    pub app_id: usize,
}


pub static FRAME_CHANNEL: Channel<CriticalSectionRawMutex, SubmitFrame, NUM_BUFFERS> =
    Channel::new();
