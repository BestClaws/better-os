use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use crate::system::hal::ambience::AsyncAmbientSensor;
use crate::system::hal::touch::AsyncTouch;

pub const AMBIENT_CHANNEL_SIZE: usize = 4;

// Channel to send ambient light percentage updates
pub static AMBIENT_CHANNEL: Channel<CriticalSectionRawMutex, u8, AMBIENT_CHANNEL_SIZE> =
    Channel::new();

#[embassy_executor::task]
pub(crate) async fn touch_sensor_service(
    touch: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncTouch>>,
) {

    let mut touch = touch.lock().await;

    loop {
        info!("Touch: reading");
        let (x, y, z) = touch.read_xy().await;

        info!("Touch: x: {}, y: {}, z: {}", x, y, z);

        // Small delay to avoid flooding logs
        Timer::after(Duration::from_millis(10)).await;
    }
}
