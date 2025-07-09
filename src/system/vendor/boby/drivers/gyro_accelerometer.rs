use crate::system::hal::ambient_sensor::AsyncAmbientSensor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::system::vendor::boby::drivers::ambient_sensor::AmbientSensorDriver;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;
use alloc::boxed::Box;
use async_trait::async_trait;
use core::cell::Cell;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use esp_hal::analog::adc::{Adc, AdcPin};
use esp_hal::i2c::master::I2c;
use esp_hal::interrupt::map;
use esp_hal::peripherals::{ADC1, GPIO1, GPIO3};
use esp_hal::riscv::asm::delay;
use esp_hal::Async;
use esp_hal_embassy::TimeBase;
use mpu6050_dmp::accel::AccelFullScale;
use mpu6050_dmp::calibration::CalibrationParameters;
use mpu6050_dmp::gravity::Gravity;
use mpu6050_dmp::gyro::GyroFullScale;
use mpu6050_dmp::quaternion::Quaternion;
use mpu6050_dmp::sensor_async::Mpu6050;
use mpu6050_dmp::yaw_pitch_roll::YawPitchRoll;
use rtt_target::rprintln;

const LGC: &str = module_path!();

enum InitState {
    I2c(I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>),
    GyroAccelerometer(Mpu6050<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>),
}


pub struct GyroAccelerometerDriver {
    sensor: Option<InitState>
}

impl GyroAccelerometerDriver {
    pub  fn new(i2c: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {

        Self { sensor: Some(InitState::I2c(i2c)) }

        
    }
}


#[async_trait(?Send)]
impl AsyncGyroAccelerometer for GyroAccelerometerDriver {
    
    async fn calibrate(&mut self) {
        
        let mut sensor = match self.sensor.take().unwrap() {
            InitState::I2c(i2c) => {
                // Initialize the MPU6050 sensor with the provided I2C bus
                Mpu6050::new(i2c, mpu6050_dmp::address::Address::default()).await.unwrap()
            },
            InitState::GyroAccelerometer(inner) => {
                // Already initialized, no action needed
                inner
            },
        };


        let mut delay  = embassy_time::Delay;
        sensor.initialize_dmp(&mut delay).await.unwrap();
        // Configure sensor calibration parameters
        // AccelFullScale options: G2, G4, G8, G16 (higher means larger range, lower precision)
        // GyroFullScale options: Deg250, Deg500, Deg1000, Deg2000 (degrees/second range)
        // ReferenceGravity: XN, XP, YN, YP, ZN, ZP (axis and direction of gravity during calibration)
        let mut calibration_params = CalibrationParameters::new(
            mpu6050_dmp::accel::AccelFullScale::G16,
            mpu6050_dmp::gyro::GyroFullScale::Deg2000,
            mpu6050_dmp::calibration::ReferenceGravity::ZP,
        );

        sensor.calibrate(&mut delay, &mut calibration_params).await.unwrap();


        defmt::info!("{} calibrating sensor", LGC);


        defmt::info!("{} sensor calibrated", LGC);

        self.sensor = Some(InitState::GyroAccelerometer(sensor));
    }

    async fn get_accelerometer_data(&mut self) -> (f32, f32, f32) {
        let x = self.sensor.as_mut().unwrap();
        match x {
            InitState::GyroAccelerometer(y) => {
                // Read raw accelerometer data (uncalibrated)
                // The accelerometer measures linear acceleration in three axes (X, Y, Z)
                // Values will be imprecise until calibration is performed
                let accel_data = y.accel().await.unwrap().scaled(AccelFullScale::G16);
                (accel_data.x(), accel_data.y(), accel_data.z())

            }
            _ => { (0.0, 0.0, 0.0) }
        }
        




    }

    async fn get_gyroscope_data(&mut self) -> (f32, f32, f32) {

        let x = self.sensor.as_mut().unwrap();
        match x {
            InitState::GyroAccelerometer(y) => {
                // Read raw gyroscope data (uncalibrated)
                // The gyroscope measures angular velocity in three axes (X, Y, Z)
                // Values will have drift and bias until calibration is performed

                let gyro_data = y.gyro().await.unwrap().scaled(GyroFullScale::Deg2000);
                (gyro_data.x(), gyro_data.y(), gyro_data.z())

            }
            _ => { (0.0, 0.0, 0.0) }
        }

    


    }

    async fn get_temperature_celsius(&mut self) -> u8 {
        let x = self.sensor.as_mut().unwrap();
        match x {
            InitState::GyroAccelerometer(y) => {
                y.temperature().await.unwrap().celsius() as u8
                
            }
            _ => { 0 }
        }

    }
    async fn get_yaw_pitch_roll(&mut self) -> (i32, i32, i32) {
        let x = self.sensor.as_mut().unwrap();
        match x {
            InitState::GyroAccelerometer(y) => {
                use core::f32::consts::PI;

                let mut buf = [0u8; 28];

                let len = y.get_fifo_count().await.unwrap();
                if len >= 28 {
                    let buf = y.read_fifo(&mut buf).await.unwrap();
                    let quat = Quaternion::from_bytes(&buf[..16]).unwrap().normalize();
                    let ypr = YawPitchRoll::from(quat);

                    let to_deg = |rad: f32| rad * (180.0 / PI);
                    let yaw_deg = to_deg(ypr.yaw) as i32;   // -180° to 180°
                    let pitch_deg = to_deg(ypr.pitch) as i32;; // -90° to 90°
                    let roll_deg = to_deg(ypr.roll) as i32;   // -180° to 180°


                    (yaw_deg, pitch_deg, roll_deg)

                } else {
                    info!("{}: Not enough data in FIFO: {}", LGC, len);
                    (0, 0, 0)
                }


            }
            _ => { (0,0,0) }
        }

    }
}
