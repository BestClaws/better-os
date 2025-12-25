use alloc::boxed::Box;
use defmt::{debug, trace, warn};
use embassy_executor::task;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};

use crate::system::hal::button::{AsyncButton, ButtonState};
use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
use crate::system::hal::touch::AsyncTouch;
use crate::system::input::bus;
use crate::system::input::devices::{
    button::map_state as map_button_state,
    encoder::map_state as map_encoder_state,
    touch::{TouchProcessor, TouchSample},
};
use crate::system::kernel::config::resources::{
    FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR,
};

const ENCODER_COOLDOWN_MS: u64 = 200;
const TOUCH_POLL_MS: u64 = 10;
const TOUCH_COALESCE_THRESHOLD: i32 = 1;

#[task]
pub async fn button_reader_task(
    button: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>,
) {
    loop {
        let state = {
            let mut button = button.lock().await;
            button.next().await
        };

        dispatch_raw_event(map_button_state(state)).await;
    }
}

#[task]
pub async fn encoder_reader_task(
    encoder: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>,
) {
    loop {
        let Ok(state) = ({
            let mut encoder = encoder.lock().await;
            encoder.next().await
        }) else {
            warn!("Failed to read encoder state");
            continue;
        };

        dispatch_raw_event(map_encoder_state(state)).await;
        Timer::after(Duration::from_millis(ENCODER_COOLDOWN_MS)).await;
    }
}

#[task]
pub async fn touch_reader_task(
    touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>,
) {
    let mut processor = TouchProcessor::new(
        FRAME_BUFFER_WIDTH,
        FRAME_BUFFER_HEIGHT,
        FRAME_SCALE_FACTOR,
        TOUCH_COALESCE_THRESHOLD,
    );
    let mut last_log = Instant::now();

    loop {
        let read_start = Instant::now();
        let sample = {
            let mut touch = touch.lock().await;
            let (x, y, pressure) = touch.read_xyz().await;
            TouchSample {
                x: x as i32,
                y: y as i32,
                pressure: pressure as i32,
            }
        };
        let read_duration = read_start.elapsed();

        if let Some(event) = processor.process_sample(sample) {
            dispatch_raw_event(event).await;
        }

        log_touch_metrics(&processor, read_duration, &mut last_log);
        Timer::after(Duration::from_millis(TOUCH_POLL_MS)).await;
    }
}

async fn dispatch_raw_event(event: crate::system::input::types::HighLevelEvent) {
    bus::raw_events().send(event).await;
}

fn log_touch_metrics(
    processor: &TouchProcessor,
    read_duration: embassy_time::Duration,
    last_log: &mut embassy_time::Instant,
) {
    let pressed = processor.last_pointer().is_some();
    if read_duration.as_millis() > 20 || (pressed && last_log.elapsed().as_secs() >= 2) {
        if let Some(pointer) = processor.last_pointer() {
            debug!(
                "Touch read: {}ms, pressed={}, pos=({},{})",
                read_duration.as_millis(),
                pressed,
                pointer.x,
                pointer.y
            );
        } else {
            debug!(
                "Touch read: {}ms, pressed={}, pos=(-,-)",
                read_duration.as_millis(),
                pressed
            );
        }
        *last_log = Instant::now();
    } else {
        trace!("Touch read latency: {}ms", read_duration.as_millis());
    }
}
