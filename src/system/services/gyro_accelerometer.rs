use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::system::services::human_input::{HumanInputEvent, INPUT_CHANNEL};

#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {

    {
        sensor.lock().await.calibrate().await;

    }


    loop {

        // let acc = sensor.lock().await.get_accelerometer_data().await;
        // defmt::info!(" Accelerometer Sensor: {:?}", acc);

        // let gyro = sensor.lock().await.get_gyroscope_data().await;
        // let temp = sensor.lock().await.get_temperature_celsius().await;
        let ypr = sensor.lock().await.get_yaw_pitch_roll().await;
        defmt::info!("Yaw: {}, Pitch: {}, Roll: {}", ypr.0, ypr.1, ypr.2);

        //
        // let gv = sensor.lock().await.get_gravity_vector().await;
        // defmt::info!(" gravity: {}, {}, {}", gv.0, gv.1, gv.2 );

        // defmt::info!("Gyro  Sensor: {:?}", gyro);
        // defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);



        Timer::after_millis(100).await;


    }
}



