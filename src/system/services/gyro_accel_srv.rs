use alloc::boxed::Box;
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

    loop {

        // let acc = sensor.lock().await.get_accelerometer_data().await;
        // defmt::info!(" Accelerometer Sensor: {:?}", acc);

        // let gyro = sensor.lock().await.get_gyroscope_data().await;
        // defmt::info!("Gyro  Sensor: {:?}", gyro);
        // let temp = sensor.lock().await.get_temperature_celsius().await;
        // defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);
        //
        let (qw, qx, qy, qz) = sensor.lock().await.get_roatation_quat().await;
        let rotated_direction = Quaternion { w: qw, x: qx, y:qy, z: qz }.rotate_vector(Vec3(0.0, 0.0, 1.0));

        defmt::info!("q: {}, {}, {}, {}", qw, qx, qy, qz);


        //
        // let gv = sensor.lock().await.get_gravity_vector().await;
        // defmt::info!(" gravity: {}, {}, {}", gv.0, gv.1, gv.2 );



        let _ = sender.send(rotated_direction).await;


        // Timer::after_millis(100).await;


    }
}





fn format_f32_8(mut x: f32) -> heapless::String<16> {
    use core::fmt::Write;
    let mut s = heapless::String::<16>::new();
    // Clamp extreme small values to zero to avoid "-0.00000000"
    if x.abs() < 0.000000005 {
        x = 0.0;
    }
    let int_part = x.trunc() as i32;
    let frac_part = ((x.abs() - x.abs().trunc()) * 100_000_000.0).round() as u32;
    // Ensure 8 digits of fractional part
    write!(s, "{}.{:08}", int_part, frac_part).unwrap();
    s
}
