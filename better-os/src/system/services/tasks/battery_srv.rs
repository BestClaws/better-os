use crate::system::hal::battery::AsyncBattery;
use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;

pub const BATTERY_CHANNEL_SIZE: usize = 4;

// Channel to send battery percentage updates
pub static BATTERY_CHANNEL: Channel<CriticalSectionRawMutex, u8, BATTERY_CHANNEL_SIZE> =
    Channel::new();

#[embassy_executor::task]
pub(crate) async fn battery_service(
    sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncBattery>>,
) {
    let sender = BATTERY_CHANNEL.sender();

    loop {
        // // Read battery percentage
        // let percent = {
        //     let mut s = sensor.lock().await;
        //     s.percent().await
        // };

        // Send battery percentage to channel
        let _ = sender.send(10).await;

        // Poll every 10 seconds
        Timer::after_millis(100).await;
    }
}
