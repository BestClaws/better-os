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


    let sender = ORIENTATION_CHANNEL.sender();
    info!("initializing gyro accelerometer service...");
    sensor.lock().await.init().await;
    info!("gyro accelerometer service initialized");

    loop {

        info!("looping service");

        // let acc = sensor.lock().await.get_accelerometer_data().await;
        // defmt::info!(" Accelerometer Sensor: {:?}", acc);

        // let gyro = sensor.lock().await.get_gyroscope_data().await;
        // defmt::info!("Gyro  Sensor: {:?}", gyro);
        // let temp = sensor.lock().await.get_temperature_celsius().await;
        // defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);
        //
        let q = sensor.lock().await.get_orientation().await;
        let rotated_direction = Quaternion { w: q.w, x: q.x, y:q.y, z: q.z }.rotate_vector(Vec3(0.0, 0.0, 1.0));
        // info!("temp: {}", sensor.lock().await.get_temperature_celsius().await);
        info!("q: {}, {}, {}, {}, norm: {}", q.w, q.x, q.y, q.z, q.magnitude());


        //
        // let gv = sensor.lock().await.get_gravity_vector().await;
        // defmt::info!(" gravity: {}, {}, {}", gv.0, gv.1, gv.2 );



        let _ = sender.send(rotated_direction).await;


        // Timer::after_millis(100).await;


    }
}

