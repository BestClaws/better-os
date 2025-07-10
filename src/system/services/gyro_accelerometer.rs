use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;

#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {




    loop {

        // let acc = sensor.lock().await.get_accelerometer_data().await;
        // defmt::info!(" Accelerometer Sensor: {:?}", acc);

        // let gyro = sensor.lock().await.get_gyroscope_data().await;
        // defmt::info!("Gyro  Sensor: {:?}", gyro);
        // let temp = sensor.lock().await.get_temperature_celsius().await;
        // defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);

        let ypr = sensor.lock().await.pitch_roll_yaw().await;
        let to_degrees = |rad: f32| rad * 180.0 / core::f32::consts::PI;
        defmt::info!("pitch: {}, yaw: {}, roll: {}", ypr.0, ypr.1, ypr.2);

        //
        // let gv = sensor.lock().await.get_gravity_vector().await;
        // defmt::info!(" gravity: {}, {}, {}", gv.0, gv.1, gv.2 );





        Timer::after_millis(1000).await;


    }
}



