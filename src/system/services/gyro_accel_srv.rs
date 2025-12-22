use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

pub static ORIENTATION_CHANNEL: Signal<CriticalSectionRawMutex, Quaternion> = Signal::new();

#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(
    sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>,
) {
    info!("initializing gyro accelerometer service...");
    let mut sensor_g = sensor.lock().await;
    sensor_g.init().await.unwrap();
    info!("gyro accelerometer service initialized");

    loop {
        let (x, y, z) = sensor_g.read_accel().await;
        debug!("gyro: x: {}, y: {}, z: {}", x, y, z);
        // ORIENTATION_CHANNEL.signal(q);
        Timer::after_millis(50).await;
    }
}
