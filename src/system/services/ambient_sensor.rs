use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::ambient_sensor::AsyncAmbientSensor;
use crate::system::hal::button::ButtonState;
use crate::system::services::human_input::{HumanInputEvent, INPUT_CHANNEL};

#[embassy_executor::task]
pub(crate) async fn ambient_sensor_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAmbientSensor>>) {



    loop {
        // let state = {
        //     let mut e = button.lock().await;
        //     e.next().await
        // };
        //
        // match state {
        //     ButtonState::Up => INPUT_CHANNEL.send(HumanInputEvent::OkReleased).await,
        //     ButtonState::Down => INPUT_CHANNEL.send(HumanInputEvent::OkPressed).await,
        //     ButtonState::Held => INPUT_CHANNEL.send(HumanInputEvent::OkHeld).await,
        // };
        let percent = {
            let mut s = sensor.lock().await;
            s.percent().await
        };

        // defmt::info!("Ambient Sensor: {}%", percent);
        Timer::after_millis(10000).await;


    }
}



