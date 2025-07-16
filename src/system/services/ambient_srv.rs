use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;
use crate::system::hal::ambience::AsyncAmbientSensor;

pub const AMBIENT_CHANNEL_SIZE: usize = 4;

// Channel to send ambient light percentage updates
pub static AMBIENT_CHANNEL: Channel<CriticalSectionRawMutex, u8, AMBIENT_CHANNEL_SIZE> =
    Channel::new();

#[embassy_executor::task]
pub(crate) async fn ambient_sensor_service(
    sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncAmbientSensor>>,
) {
    let sender = AMBIENT_CHANNEL.sender();

    loop {
        let percent = {
            let mut s = sensor.lock().await;
            s.percent().await
        };

        let _ = sender.send(percent).await;

        // Poll interval (adjust if needed)
        Timer::after_millis(1000).await;
    }
}
