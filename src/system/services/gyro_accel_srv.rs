use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use micromath::F32Ext;
use crate::system::vendor::invensense::drivers::mpu6050::sensor::{get_gravity, get_yaw_pitch_roll, MPU6050};

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
        info!("yaw: {}, pitch: {}, roll: {}", ypr.0 * 57.296, ypr.1 * 57.296, ypr.2 * 57.296,);
        let rotated_direction = q.rotate_vector(Vec3(0.0, 0.0, 1.0));
        let _ = sender.send(rotated_direction).await;
        Timer::after_millis(100).await;






    }
}

