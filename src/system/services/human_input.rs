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
}

pub(crate) static HUMAN_INPUT_CH: Channel<CriticalSectionRawMutex, HumanInputEvent, 8> = Channel::new();


pub(crate) mod sub {
    use alloc::boxed::Box;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::mutex::Mutex;
    use crate::system::hal::button::{AsyncButton, ButtonState};
    use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
    use crate::system::services::human_input::{HumanInputEvent, HUMAN_INPUT_CH};

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

            match state {
                EncoderState::Ccw => HUMAN_INPUT_CH.send(HumanInputEvent::NavUp).await,
                EncoderState::Cw => HUMAN_INPUT_CH.send(HumanInputEvent::NavDown).await,
            };
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



}



