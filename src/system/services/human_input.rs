use alloc::boxed::Box;
use core::ops::Deref;
use defmt::{info};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use crate::system::hal::button::{AsyncButton, ButtonState};
use crate::system::hal::encoder::{AsyncEncoder, EncoderState};


pub(crate) enum HumanInputEvent {
    NavUp,
    NavDown,
    OK_PRESSED,
    OK_RELEASED,
    OK_HELD,
}

pub(crate) static INPUT_CHANNEL: Channel<CriticalSectionRawMutex, HumanInputEvent, 8> = Channel::new();

// TODO: why is there a reference to driver in the task? get rid of this.
#[embassy_executor::task]
pub async fn human_input_service(spawner: &'static mut Spawner, e: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>, b: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>) {
    info!("[{}s] human input service started", Instant::now().as_millis() as f32 / 1000f32);



    loop {


    }
}

pub(crate) mod sub {
    use alloc::boxed::Box;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::mutex::Mutex;
    use crate::system::hal::button::{AsyncButton, ButtonState};
    use crate::system::hal::encoder::{AsyncEncoder, EncoderState};
    use crate::system::services::human_input::{HumanInputEvent, INPUT_CHANNEL};

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
                EncoderState::Ccw => INPUT_CHANNEL.send(HumanInputEvent::NavUp).await,
                EncoderState::Cw => INPUT_CHANNEL.send(HumanInputEvent::NavDown).await,
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
                ButtonState::Up => INPUT_CHANNEL.send(HumanInputEvent::OK_RELEASED).await,
                ButtonState::Down => INPUT_CHANNEL.send(HumanInputEvent::OK_PRESSED).await,
                ButtonState::Held => INPUT_CHANNEL.send(HumanInputEvent::OK_HELD).await,
            };
        }
    }



}



