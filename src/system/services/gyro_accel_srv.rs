use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use crate::system::vendor::invensense::drivers::mpu6050::sensor::{get_gravity, get_yaw_pitch_roll};

pub static ORIENTATION_CHANNEL: Signal<CriticalSectionRawMutex, Quaternion> =
    Signal::new();


#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {

    info!("initializing gyro accelerometer service...");
    let mut sensor_g = sensor.lock().await;
    sensor_g.init().await;
    info!("gyro accelerometer service initialized");


    loop {
        let q = sensor_g.get_orientation().await;
        ORIENTATION_CHANNEL.signal(q);
        Timer::after_millis(50).await;

    }
}

