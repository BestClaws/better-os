use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};

pub static ORIENTATION_CHANNEL: Channel<CriticalSectionRawMutex, Vec3, 10> =
    Channel::new();

pub static ACCEL_VECTOR_CH: Channel<CriticalSectionRawMutex, Vec3, 10> =
    Channel::new();



#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {


    let sender = ACCEL_VECTOR_CH.sender();

    loop {

        let acc = sensor.lock().await.get_acceleration().await;
        defmt::info!(" Accelerometer Sensor: {:?}", acc);

        // let gyro = sensor.lock().await.get_gyroscope_data().await;
        // defmt::info!("Gyro  Sensor: {:?}", gyro);
        // let temp = sensor.lock().await.get_temperature_celsius().await;
        // defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);
        //
        // let (qw, qx, qy, qz) = sensor.lock().await.get_roatation_quat().await;
        // let rotated_direction = Quaternion { w: qw, x: qx, y:qy, z: qz }.rotate_vector(Vec3(0.0, 0.0, 1.0));


        //
        // let gv = sensor.lock().await.get_gravity_vector().await;
        // defmt::info!(" gravity: {}, {}, {}", gv.0, gv.1, gv.2 );



        let _ = sender.send(Vec3(acc.0, acc.1, acc.2).normalize()).await;


        // Timer::after_millis(100).await;


    }
}





