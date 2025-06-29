// src/mpu6050.rs
use embedded_hal_async::i2c::I2c;
use micromath::F32Ext;

const MPU6050_ADDR_DEFAULT: u8 = 0x68; // Default I2C address (AD0 low)
const PWR_MGMT_1: u8 = 0x6B; // Power management register
const CONFIG: u8 = 0x1A; // Configuration register for digital low-pass filter
const ACCEL_CONFIG: u8 = 0x1C; // Accelerometer configuration
const GYRO_CONFIG: u8 = 0x1B; // Gyroscope configuration
const SMPLRT_DIV: u8 = 0x19; // Sample rate divider
const ACCEL_XOUT_H: u8 = 0x3B; // Start of accelerometer data registers

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Orientation {
    FaceUp,
    FaceDown,
    PortraitUp,
    PortraitDown,
    LandscapeLeft,
    LandscapeRight,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format, Default)]
pub struct SensorData {
    pub ax: i16, // Accelerometer X-axis
    pub ay: i16, // Accelerometer Y-axis
    pub az: i16, // Accelerometer Z-axis
    pub gx: i16, // Gyroscope X-axis
    pub gy: i16, // Gyroscope Y-axis
    pub gz: i16, // Gyroscope Z-axis
}

/// MPU6050 driver for reading accelerometer and gyroscope data.
pub struct Mpu6050<I2C> {
    i2c: I2C,
    address: u8, // I2C address of the MPU6050
    tolerance: SensorData, // Threshold for clamping small jitters to zero
    reference: Option<(i16, i16, i16)>, // Reference pose vector for orientation
    initialized: bool, // Tracks initialization state
}

impl<I2C> Mpu6050<I2C>
where
    I2C: I2c,
{
    /// Creates a new MPU6050 instance with the given I2C interface and address.
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            address: MPU6050_ADDR_DEFAULT,
            tolerance: SensorData::default(),
            reference: None,
            initialized: false,
        }
    }

    /// Initializes the MPU6050 sensor by configuring power, filter, and ranges.
    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        // Wake up the sensor (disable sleep mode)
        if let Err(e) = self.enable_sensor().await {
            defmt::error!("Failed to enable sensor: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }

        // Configure digital low-pass filter (DLPF to 44Hz)
        if let Err(e) = self.configure_filter().await {
            defmt::error!("Failed to configure filter: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }

        // Configure accelerometer (±2g) and gyroscope (±250°/s)
        if let Err(e) = self.i2c.write(self.address, &[ACCEL_CONFIG, 0x00]).await {
            defmt::error!("Failed to configure accelerometer: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }
        if let Err(e) = self.i2c.write(self.address, &[GYRO_CONFIG, 0x00]).await {
            defmt::error!("Failed to configure gyroscope: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }

        // Set sample rate to 100Hz (8kHz / (1 + 79))
        if let Err(e) = self.i2c.write(self.address, &[SMPLRT_DIV, 79]).await {
            defmt::error!("Failed to configure sample rate: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }

        // Wait for sensor stabilization
        embassy_time::Timer::after_millis(100).await;

        // Compute baseline and tolerance
        let baseline = match self.read_avg(50).await {
            Ok(data) => data,
            Err(e) => {
                defmt::error!("Failed to read baseline: {:?}", defmt::Debug2Format(&e));
                return Err(e);
            }
        };
        self.tolerance = self.compute_tolerance(&baseline);

        // Calibrate reference pose
        if let Err(e) = self.recalibrate_pose().await {
            defmt::error!("Failed to calibrate pose: {:?}", defmt::Debug2Format(&e));
            return Err(e);
        }

        self.initialized = true;
        defmt::info!("MPU6050 initialized successfully");
        Ok(())
    }

    /// Wakes the sensor by clearing the sleep bit in PWR_MGMT_1.
    async fn enable_sensor(&mut self) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[PWR_MGMT_1, 0x00]).await
    }

    /// Configures the digital low-pass filter (DLPF) to 44Hz.
    async fn configure_filter(&mut self) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[CONFIG, 0x03]).await
    }

    /// Computes tolerance thresholds for noise deadzone based on baseline data.
    fn compute_tolerance(&self, baseline: &SensorData) -> SensorData {
        let clamp = |v: i16| -> i16 { ((v as i32).abs() / 2).max(200) as i16 };
        SensorData {
            ax: clamp(baseline.ax),
            ay: clamp(baseline.ay),
            az: clamp(baseline.az),
            gx: clamp(baseline.gx),
            gy: clamp(baseline.gy),
            gz: clamp(baseline.gz),
        }
    }

    /// Applies a deadzone to filter out small jitter/noise.
    fn apply_deadzone(val: i16, threshold: i16) -> i16 {
        if val.abs() < threshold {
            0
        } else {
            val
        }
    }

    /// Reads and applies deadzone to all sensor data.
    pub async fn read_all(&mut self) -> Result<SensorData, I2C::Error> {
        let raw = self.read_raw_data().await?;

        Ok(SensorData {
            ax: Self::apply_deadzone(raw.ax, self.tolerance.ax),
            ay: Self::apply_deadzone(raw.ay, self.tolerance.ay),
            az: Self::apply_deadzone(raw.az, self.tolerance.az),
            gx: Self::apply_deadzone(raw.gx, self.tolerance.gx),
            gy: Self::apply_deadzone(raw.gy, self.tolerance.gy),
            gz: Self::apply_deadzone(raw.gz, self.tolerance.gz),
        })
    }

    /// Reads raw accelerometer and gyroscope data from the sensor.
    async fn read_raw_data(&mut self) -> Result<SensorData, I2C::Error> {
        let mut buf = [0u8; 14];
        self.i2c.write_read(self.address, &[ACCEL_XOUT_H], &mut buf).await?;

        Ok(SensorData {
            ax: i16::from_be_bytes([buf[0], buf[1]]),
            ay: i16::from_be_bytes([buf[2], buf[3]]),
            az: i16::from_be_bytes([buf[4], buf[5]]),
            gx: i16::from_be_bytes([buf[8], buf[9]]),
            gy: i16::from_be_bytes([buf[10], buf[11]]),
            gz: i16::from_be_bytes([buf[12], buf[13]]),
        })
    }

    /// Reads averaged sensor data over multiple samples.
    pub async fn read_avg(&mut self, samples: usize) -> Result<SensorData, I2C::Error> {
        let mut sum = (0i32, 0, 0, 0, 0, 0);

        for _ in 0..samples {
            let raw = self.read_raw_data().await?;
            sum.0 += raw.ax as i32;
            sum.1 += raw.ay as i32;
            sum.2 += raw.az as i32;
            sum.3 += raw.gx as i32;
            sum.4 += raw.gy as i32;
            sum.5 += raw.gz as i32;
            embassy_time::Timer::after_micros(200).await; // Reduced delay
        }

        Ok(SensorData {
            ax: (sum.0 / samples as i32) as i16,
            ay: (sum.1 / samples as i32) as i16,
            az: (sum.2 / samples as i32) as i16,
            gx: (sum.3 / samples as i32) as i16,
            gy: (sum.4 / samples as i32) as i16,
            gz: (sum.5 / samples as i32) as i16,
        })
    }

    /// Detects the sensor's orientation relative to the reference pose.
    pub async fn detect_orientation(&mut self) -> Result<Orientation, I2C::Error> {
        if !self.initialized {
            defmt::warn!("Sensor not initialized; call init() first");
            return Ok(Orientation::Unknown);
        }

        let SensorData { ax, ay, az, .. } = self.read_all().await?;
        let ref_vec = match self.reference {
            Some(r) => r,
            None => {
                defmt::warn!("No reference vector set");
                return Ok(Orientation::Unknown);
            }
        };

        let normalize = |x: i16, y: i16, z: i16| -> Option<(f32, f32, f32)> {
            let xf = x as f32 / 16384.0; // ±2g range, 16-bit resolution
            let yf = y as f32 / 16384.0;
            let zf = z as f32 / 16384.0;
            let mag_sq = xf * xf + yf * yf + zf * zf;
            if mag_sq < 0.1 { // Avoid division by near-zero
                return None;
            }
            let mag = mag_sq.sqrt();
            Some((xf / mag, yf / mag, zf / mag))
        };

        let ref_norm = match normalize(ref_vec.0, ref_vec.1, ref_vec.2) {
            Some(v) => v,
            None => return Ok(Orientation::Unknown),
        };
        let curr_norm = match normalize(ax, ay, az) {
            Some(v) => v,
            None => return Ok(Orientation::Unknown),
        };

        let dot = ref_norm.0 * curr_norm.0 + ref_norm.1 * curr_norm.1 + ref_norm.2 * curr_norm.2;

        // Adjusted thresholds for better stability
        if dot > 0.85 {
            Ok(Orientation::FaceUp)
        } else if dot < -0.85 {
            Ok(Orientation::FaceDown)
        } else if curr_norm.0.abs() > curr_norm.1.abs() && curr_norm.0.abs() > 0.5 {
            if curr_norm.0 > 0.0 {
                Ok(Orientation::LandscapeRight)
            } else {
                Ok(Orientation::LandscapeLeft)
            }
        } else if curr_norm.1.abs() > 0.5 {
            if curr_norm.1 > 0.0 {
                Ok(Orientation::PortraitUp)
            } else {
                Ok(Orientation::PortraitDown)
            }
        } else {
            Ok(Orientation::Unknown)
        }
    }

    /// Recalibrates the reference pose using averaged accelerometer data.
    pub async fn recalibrate_pose(&mut self) -> Result<(), I2C::Error> {
        let SensorData { ax, ay, az, .. } = self.read_avg(25).await?;
        self.reference = Some((ax, ay, az));
        defmt::info!("Reference pose updated: ({}, {}, {})", ax, ay, az);
        Ok(())
    }
}