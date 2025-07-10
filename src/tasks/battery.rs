use embassy_sync::semaphore::Semaphore;

use crate::{
    system::resources::framebuffer::{request_framebuffer, FB_SEMAPHORE, FRAME_CHANNEL, SubmitFrame},
};

#[embassy_executor::task]
pub async fn battery() {
    // Wait for a free framebuffer
    FB_SEMAPHORE.acquire(1).await.unwrap();

    if let Some(fb) = request_framebuffer() {
        draw_ui(fb.buf);

        FRAME_CHANNEL.sender().send(SubmitFrame {
            id: fb.id,
            app_id: 2, // unique per app
        }).await;


        core::mem::forget(fb); // compositor owns & drops it
    }
}

fn draw_ui(buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = if (i / 16) % 2 == 0 {
            if i % 2 == 0 { 0xF0 } else { 0x0F }
        } else {
            if i % 2 == 0 { 0x0F } else { 0xF0 }
        };
    }
}



