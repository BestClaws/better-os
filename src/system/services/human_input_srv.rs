use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;


#[derive(Format)]
pub(crate) enum HumanInputEvent {
    NavUp,
    NavDown,
    OkPressed,
    OkReleased,
    OkHeld,
    Touch(i32, i32), // NEW: Touch event with normalized coordinates
}


pub(crate) static HUMAN_INPUT_CH: Channel<CriticalSectionRawMutex, HumanInputEvent, 30> = Channel::new();


pub(crate) mod sub {
    use alloc::boxed::Box;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::mutex::Mutex;
    use defmt::info;
    use embassy_time::Timer;
    use crate::system::hal::button::{AsyncButton, ButtonState};
    use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
    use crate::system::hal::touch::AsyncTouch;
    use crate::system::kernel::config::resources::{FRAME_BUFFER_HEIGHT, FRAME_BUFFER_WIDTH, FRAME_SCALE_FACTOR};
    use crate::system::services::human_input_srv::{HumanInputEvent, HUMAN_INPUT_CH};

    #[embassy_executor::task]
    pub(crate) async fn listen_encoder(encoder: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>) {
        loop {
            let Ok(state) = ({
                let mut e = encoder.lock().await;
                e.next().await
            }) else {
                defmt::error!("Failed to read encoder state");
                continue; // Skip this iteration if there's an error
            };

            info!("Encoder state: {:?}", state);

            match state {
                EncoderState::Ccw => HUMAN_INPUT_CH.send(HumanInputEvent::NavUp).await,
                EncoderState::Cw => HUMAN_INPUT_CH.send(HumanInputEvent::NavDown).await,
            };

            Timer::after_millis(500).await; // cooldown
            info!("Cooldown done");
        }
    }

    #[embassy_executor::task]
    pub(crate) async fn listen_button(button: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>) {
        loop {
            let state = {
                let mut e = button.lock().await;
                e.next().await
            };

            match state {
                ButtonState::Up => HUMAN_INPUT_CH.send(HumanInputEvent::OkReleased).await,
                ButtonState::Down => HUMAN_INPUT_CH.send(HumanInputEvent::OkPressed).await,
                ButtonState::Held => HUMAN_INPUT_CH.send(HumanInputEvent::OkHeld).await,
            };
        }
    }


    #[embassy_executor::task]
    pub(crate) async fn listen_touch(touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>) {




        let x_start = 310;
        let y_start = 370;

        let x_end = 3775.0;
        let x_highest = x_end - x_start as f32;
        let y_end = 3797.0;
        let y_highest = y_end - y_start as f32;

        loop {
            let (x, y, z) = {
                let mut t = touch.lock().await;
                t.read_xyz().await
            };



            info!("------------------------------------------------Touch detected: x = {}, y = {}, z = {}", x, y, z);

            // Touch pressure threshold (ignore light/noisy touches)
            if z > 0 {

                let calibrated_x = x.saturating_sub(x_start) as f32;
                let calibrated_y = y.saturating_sub(y_start) as f32;
                // Normalize/clamp x and y to a max of 2000
                let y1 = ((calibrated_x / x_highest as f32) * (240 / FRAME_SCALE_FACTOR) as f32)  as i32;
                let x1 = ((calibrated_y / y_highest as f32) * (320 / FRAME_SCALE_FACTOR) as f32) as i32;


                if x1 > FRAME_BUFFER_WIDTH as i32 || y1 > FRAME_BUFFER_HEIGHT as i32 || x1 == 0  || y1 == 0 {
                    continue;
                }
                info!("touch: x = {}({} / {} * ({} / {})), y = {}({} / {} * ({} / {}))", x1, calibrated_y, y_highest, 320, FRAME_SCALE_FACTOR, y1, calibrated_x, x_highest, 240,  FRAME_SCALE_FACTOR);
                HUMAN_INPUT_CH.send(HumanInputEvent::Touch(x1, y1)).await;
            }

            Timer::after_millis(16).await;
        }
    }



}



