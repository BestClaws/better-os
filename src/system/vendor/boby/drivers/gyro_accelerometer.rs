#![allow(unused)]



use micromath::F32Ext;
use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::{info, unwrap};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use esp_hal::riscv::asm::delay;
use mpu6050_dmp::accel::{Accel, AccelFullScale};
use mpu6050_dmp::calibration::{CalibrationParameters, ReferenceGravity};
use mpu6050_dmp::gyro::{Gyro, GyroFullScale};
use mpu6050_dmp::quaternion::Quaternion;
use mpu6050_dmp::sensor_async::Mpu6050;
use mpu6050_dmp::yaw_pitch_roll::YawPitchRoll;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;

const LGC: &str = module_path!();

enum InitState {
    I2c(I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>),
    GyroAccelerometer(Mpu6050<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>),
}


pub struct GyroAccelerometerDriver {
    sensor: Option<InitState>
}

impl GyroAccelerometerDriver {
    pub fn new(i2c: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {
        Self { sensor: Some(InitState::I2c(i2c)) }
    }

    async fn get_sensor(
        &mut self,
    ) -> &mut Mpu6050<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>> {
        // Case 1: Already initialized → return directly
        if let Some(InitState::GyroAccelerometer(ref mut sensor)) = self.sensor {
            return sensor;
        }

        // Case 2: Take I2C and init
        let InitState::I2c(i2c) = self.sensor.take().unwrap() else {
            panic!("Sensor already initialized but incorrectly");
        };

        let mut sensor = Mpu6050::new(i2c, mpu6050_dmp::address::Address::default())
            .await
            .unwrap();

        embassy_time::Timer::after_millis(2000).await;



        sensor.set_sample_rate_divider(99).await.unwrap(); // 100Hz
        sensor.enable_fifo().await.unwrap();

        loop {
            let result = sensor.initialize_dmp(&mut embassy_time::Delay).await;
            if let Ok(()) = result {
                info!("{}: DMP initialized successfully", LGC);
                break;
            } else {
                info!("{}: DMP initialization failed retrying", LGC);
                embassy_time::Timer::after_millis(100).await;

                continue
            }
        }

        // let calibration = CalibrationParameters::new(
        //     AccelFullScale::G8,
        //     GyroFullScale::Deg1000,
        //     ReferenceGravity::ZP,
        // );
        //
        // loop {
        //     let result = sensor.calibrate(&mut embassy_time::Delay, &calibration).await;
        //     if let Ok((a, g)) = result {
        //         sensor.set_accel_calibration(&Accel::new( 1421,  739,  870));
        //         sensor.set_gyro_calibration(&Gyro::new(  26, -36, 7));
        //         info!("Calibration successful: Accel: {:?}, Gyro: {:?}", a, g);
        //         break; // Calibration successful
        //     } else {
        //         info!("{}: Calibration failed retrying", LGC);
        //         embassy_time::Timer::after_millis(100).await;
        //         continue
        //     }
        // }

        sensor.set_accel_calibration(&Accel::new( 1421,  739,  870));
        sensor.set_gyro_calibration(&Gyro::new(  26, -36, 7));


        // Re-insert into self.sensor and get reference
        self.sensor = Some(InitState::GyroAccelerometer(sensor));

        // Safe unwrap, we just put it there
        match self.sensor {
            Some(InitState::GyroAccelerometer(ref mut sensor)) => sensor,
            _ => unreachable!(),
        }
    }
}


#[async_trait(?Send)]
impl AsyncGyroAccelerometer for GyroAccelerometerDriver {
    async fn get_acceleration(&mut self) -> (f32, f32, f32) {
        let sensor = self.get_sensor().await;
        loop {
            let result = sensor.accel().await;
            if let Ok(accel_data) = result {
                let accel_data = accel_data.scaled(AccelFullScale::G2);
                return (accel_data.x(), accel_data.y(), accel_data.z());
            } else {
                continue;
            }

        }
    }

    async fn get_gyroscope_data(&mut self) -> (f32, f32, f32) {
        let sensor = self.get_sensor().await;
        let gyro_data = sensor.gyro().await.unwrap().scaled(GyroFullScale::Deg1000);
        (gyro_data.x(), gyro_data.y(), gyro_data.z())
    }

    async fn get_temperature_celsius(&mut self) -> u8 {
        self.get_sensor().await.temperature().await.unwrap().celsius() as u8
    }

    async fn get_roatation_quat(&mut self) -> (f32, f32, f32, f32) {
        let sensor = self.get_sensor().await;
        // Buffer for FIFO data (DMP packets are 28 bytes)
        let mut buffer = [0u8; 28];



            loop {

                let Ok(fifo_count) = sensor.get_fifo_count().await else {
                    embassy_time::Timer::after_millis(1).await;
                    continue;
                };

                if fifo_count >= 28 {
                    // Read a complete DMP packet
                    let res = sensor.read_fifo(&mut buffer).await;
                    if let Ok(data) = res {
                        // First 16 bytes contain quaternion data
                        // The quaternion represents the sensor's orientation in 3D space:
                        // - w: cos(angle/2) - indicates amount of rotation
                        // - x,y,z: axis * sin(angle/2) - indicates rotation axis
                        let q = Quaternion::from_bytes(&data[..16]).unwrap().normalize();
                        return (q.w, q.x, q.y, q.z);
                    } else {
                        embassy_time::Timer::after_millis(1).await;
                        continue;
                    }

                } else {
                    embassy_time::Timer::after_millis(1).await;
                    continue
                }

            }




    }
}


