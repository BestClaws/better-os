use micromath::F32Ext;
use alloc::boxed::Box;
use async_trait::async_trait;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use mpu6050_dmp::accel::AccelFullScale;
use mpu6050_dmp::calibration::{CalibrationParameters, ReferenceGravity};
use mpu6050_dmp::gyro::GyroFullScale;
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
            panic!("Sensor already initialized incorrectly");
        };

        let mut sensor = Mpu6050::new(i2c, mpu6050_dmp::address::Address::default())
            .await
            .unwrap();

        let mut delay = embassy_time::Delay;
        sensor.initialize_dmp(&mut delay).await.unwrap();

        let mut calibration = CalibrationParameters::new(
            AccelFullScale::G8,
            GyroFullScale::Deg1000,
            ReferenceGravity::ZP,
        );

        let _ = sensor.calibrate(&mut delay, &mut calibration).await.unwrap();

        sensor.set_sample_rate_divider(99).await.unwrap(); // 100Hz
        sensor.enable_fifo().await.unwrap();

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
    
    async fn calibrate(&mut self) {








    }



    async fn get_accelerometer_data(&mut self) -> (f32, f32, f32) {
        let sensor = self.get_sensor().await;
        // Read raw accelerometer data (uncalibrated)
        // The accelerometer measures linear acceleration in three axes (X, Y, Z)
        // Values will be imprecise until calibration is performed
        let accel_data = sensor.accel().await.unwrap().scaled(AccelFullScale::G8);
        (accel_data.x(), accel_data.y(), accel_data.z())

    }

    async fn get_gyroscope_data(&mut self) -> (f32, f32, f32) {
        let sensor = self.get_sensor().await;
        let gyro_data = sensor.gyro().await.unwrap().scaled(GyroFullScale::Deg1000);
        (gyro_data.x(), gyro_data.y(), gyro_data.z())


    }

    async fn get_temperature_celsius(&mut self) -> u8 {
        self.get_sensor().await.temperature().await.unwrap().celsius() as u8
    }

    async fn pitch_roll_yaw(&mut self) -> (i16, i16, i16) {
        let sensor = self.get_sensor().await;
        // Buffer for FIFO data (DMP packets are 28 bytes)
        let mut buffer = [0u8; 28];
        loop {
            let fifo_count = sensor.get_fifo_count().await.unwrap();

            if fifo_count >= 28 {
                // Read a complete DMP packet
                let data = sensor.read_fifo(&mut buffer).await.unwrap();
                // First 16 bytes contain quaternion data
                // The quaternion represents the sensor's orientation in 3D space:
                // - w: cos(angle/2) - indicates amount of rotation
                // - x,y,z: axis * sin(angle/2) - indicates rotation axis
                let q = Quaternion::from_bytes(&data[..16]).unwrap().normalize();
                let ypr = YawPitchRoll::from(q);

                // Convert radians to degrees for more intuitive reading
                let yaw_deg = ypr.yaw * 180.0 / core::f32::consts::PI;
                let pitch_deg = ypr.pitch * 180.0 / core::f32::consts::PI;
                let roll_deg = ypr.roll * 180.0 / core::f32::consts::PI;

                // Round and clamp to nearest integer
                let yaw_rounded = yaw_deg.round() as i16;
                let pitch_rounded = pitch_deg.round() as i16;
                let roll_rounded = roll_deg.round() as i16;

                // // Format with sign, pad with zeros to always be 3 digits
                // let formatted = format!("({yaw_rounded:+04}, {pitch_rounded:+04}, {roll_rounded:+04})");
                // info!("{}", formatted.as_str());
                return (pitch_rounded, roll_rounded, yaw_rounded)
            }

        }

    }
}


