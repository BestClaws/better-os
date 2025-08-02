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
        loop {
            let (x, y, z) = {
                let mut t = touch.lock().await;
                t.read_xyz().await
            };

            // Touch pressure threshold (ignore light/noisy touches)
            if z > 0 {
                // Normalize/clamp x and y to a max of 2000
                let x = ((x as f32 / 2000.0) * (320 / FRAME_SCALE_FACTOR) as f32) as i32;
                let y = ((y as f32 / 2000.0) * (240 / FRAME_SCALE_FACTOR) as f32) as i32;

                if x > FRAME_BUFFER_WIDTH as i32 || y > FRAME_BUFFER_HEIGHT as i32 || x == 0  || y == 0 {
                    continue;
                }

                info!("Touch detected: x = {}, y = {}, z = {}", x, y, z);
                HUMAN_INPUT_CH.send(HumanInputEvent::Touch(x, y)).await;
            }

            // Optional debounce delay (adjust as needed)
            Timer::after_millis(30).await;
        }
    }



}



