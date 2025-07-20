use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Vec3};
use crate::system::vendor::invensense::drivers::mpu6050::sensor::{get_gravity, get_yaw_pitch_roll};

pub static ORIENTATION_CHANNEL: Channel<CriticalSectionRawMutex, Vec3, 1> =
    Channel::new();


#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {
    let sender = ORIENTATION_CHANNEL.sender();

    info!("initializing gyro accelerometer service...");
    sensor.lock().await.init().await;
    info!("gyro accelerometer service initialized");


    loop {
        let q = sensor.lock().await.get_orientation().await;
        // info!("q: {}, {}, {}, {} norm: {}", q.w, q.x, q.y, q.z, q.magnitude());
        let g = get_gravity(&q);
        let ypr = get_yaw_pitch_roll(&q, &g);
        let rotated_direction = q.rotate_vector(Vec3(0.0, 0.0, 1.0));
        let _ = sender.try_send(rotated_direction);
        Timer::after_millis(50).await;

    }
}

