use alloc::boxed::Box;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;

#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>) {


    sensor.lock().await.calibrate().await;

    loop {

        let acc = sensor.lock().await.get_accelerometer_data().await;
        let gyro = sensor.lock().await.get_gyroscope_data().await;
        let temp = sensor.lock().await.get_temperature_celsius().await;
        defmt::info!("Gyro Accelerometer Sensor: Acc: {:?}, Gyro: {:?}", acc, gyro);
        defmt::info!("Gyro Accelerometer Sensor: Temperature: {}C", temp);


        Timer::after_micros(1000).await;


    }
}



