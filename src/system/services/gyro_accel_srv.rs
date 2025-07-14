use alloc::boxed::Box;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use micromath::F32Ext;

pub static ORIENTATION_CHANNEL: Channel<CriticalSectionRawMutex, Vec3, 10> =
    Channel::new();


#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {


    info!("initializing gyro accelerometer service...");
    sensor.lock().await.initialize().await;
    info!("gyro accelerometer service initialized");

    loop {
        info!("looping service");


        // let q = sensor.lock().await.get_orientation().await;
        // info!("{}, {}, {}, {}, norm: {}", q.x, q.y, q.z, q.w, q.normalize());

        Timer::after_millis(100).await;


    }
}

