use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::system::input::dispatcher::{SUI_ACK_CH, SUI_EVENT_CH};
use crate::system::input::types::{HighLevelEvent, MotionEvent, TouchAction};

use super::animation::TransitionDirection;

pub static SUI_COMMAND_CH: Channel<CriticalSectionRawMutex, TransitionDirection, 4> = Channel::new();

#[embassy_executor::task]
pub async fn system_ui_consume_events() {
    use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH};
    const EDGE_THRESHOLD: i32 = 24;
    const MIN_SWIPE_DISTANCE: i32 = 60;
    const MIN_SWIPE_ANGLE_TOLERANCE: i32 = 20;
    let mut tracking = false;
    let mut start_x = 0i32; let mut start_y = 0i32; let mut from_left = false; let mut from_right = false; let mut fired = false;
    loop {
        let ev = SUI_EVENT_CH.receive().await;
        let mut consumed = false;
        if let HighLevelEvent::Motion(MotionEvent { action, pointers, .. }) = ev {
            if let Some(p) = pointers[0] {
                match action {
                    TouchAction::Down => { tracking = false; from_left = p.x <= EDGE_THRESHOLD; from_right = p.x >= ((FRAME_BUFFER_WIDTH as i32) - EDGE_THRESHOLD); if from_left || from_right { tracking = true; start_x = p.x; start_y = p.y; } }
                    TouchAction::Move => { if tracking && !fired { let dx = p.x - start_x; let dy = (p.y - start_y).abs(); if dy <= MIN_SWIPE_ANGLE_TOLERANCE { if from_left && dx > MIN_SWIPE_DISTANCE { consumed = true; fired = true; SUI_COMMAND_CH.send(TransitionDirection::Next).await; } else if from_right && (-dx) > MIN_SWIPE_DISTANCE { consumed = true; fired = true; SUI_COMMAND_CH.send(TransitionDirection::Previous).await; } } } }
                    TouchAction::Up => { tracking = false; fired = false; }
                }
            }
        }
        SUI_ACK_CH.send(consumed).await;
    }
}


