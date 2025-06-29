use embedded_hal_async::i2c::I2c;
use micromath::F32Ext;

const MPU6050_ADDR_DEFAULT: u8 = 0x68; // Default I2C address (AD0 low)
const PWR_MGMT_1: u8 = 0x6B; // Power management register
const CONFIG: u8 = 0x1A; // Configuration register for digital low-pass filter
const ACCEL_CONFIG: u8 = 0x1C; // Accelerometer configuration
const GYRO_CONFIG: u8 = 0x1B; // Gyroscope configuration
const SMPLRT_DIV: u8 = 0x19; // Sample rate divider
const ACCEL_XOUT_H: u8 = 0x3B; // Start of accelerometer data registers
const TEMP_OUT_H: u8 = 0x41; // Temperature sensor data registers

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Orientation {
    FaceUp,
    FaceDown,
    Portrait,
    PortraitInverted,
    Landscape,
    LandscapeInverted,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum AccelRange {
    G2,  // ±2g
    G4,  // ±4g
    G8,  // ±8g
    G16, // ±16g
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum GyroRange {
    Dps250,  // ±250°/s
    Dps500,  // ±500°/s
    Dps1000, // ±1000°/s
    Dps2000, // ±2000°/s
}

#[derive(Debug, defmt::Format)]
pub enum Mpu6050Error<E> {
    I2c(E),
    InvalidSamples,
    NotInitialized,
    InvalidData,
    InvalidCalibration,
}

/// MPU6050 driver for reading accelerometer, gyroscope, and temperature data.
pub struct Mpu6050<I2C> {
    i2c: I2C,
    address: u8,                       // I2C address of the MPU6050
    tolerance: SensorData,             // Threshold for clamping small jitters to zero
    reference: Option<(i16, i16, i16)>, // Reference pose vector for orientation
    initialized: bool,                 // Tracks initialization state
    accel_range: AccelRange,           // Accelerometer sensitivity
    gyro_range: GyroRange,             // Gyroscope sensitivity
    last_time: Option<u64>,            // Last timestamp for gyro integration (microseconds)
}

// Core functionality and initialization
impl<I2C> Mpu6050<I2C>
where
    I2C: I2c,
{
    /// Creates a new MPU6050 instance with optional accelerometer and gyroscope ranges.
    pub fn new(i2c: I2C, accel_range: Option<AccelRange>, gyro_range: Option<GyroRange>) -> Self {
        Self {
            i2c,
            address: MPU6050_ADDR_DEFAULT,
            tolerance: SensorData::default(),
            reference: None,
            initialized: false,
            accel_range: accel_range.unwrap_or(AccelRange::G2),
            gyro_range: gyro_range.unwrap_or(GyroRange::Dps250),
            last_time: None,
        }
    }

    /// Initializes the MPU6050 using the stored accelerometer and gyroscope ranges.
    pub async fn init(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        self.enable_sensor().await?;
        self.configure_filter().await?;
        self.configure_accelerometer().await?;
        self.configure_gyroscope().await?;
        self.configure_sample_rate().await?;

        // Wait for sensor stabilization
        embassy_time::Timer::after_millis(100).await;

        // Compute baseline and tolerance
        let baseline = self.read_avg(50).await?;
        self.tolerance = self.compute_tolerance(&baseline);

        // Calibrate reference pose
        self.recalibrate_pose().await?;

        self.initialized = true;
        self.last_time = Some(embassy_time::Instant::now().as_micros());
        defmt::info!("MPU6050 initialized: accel={:?}, gyro={:?}", self.accel_range, self.gyro_range);
        Ok(())
    }

    /// Wakes the sensor by clearing the sleep bit in PWR_MGMT_1.
    async fn enable_sensor(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        self.write_register(PWR_MGMT_1, 0x00)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(())
    }

    /// Configures the digital low-pass filter (DLPF) to 44Hz.
    async fn configure_filter(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        self.write_register(CONFIG, 0x03)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(())
    }

    /// Configures the accelerometer range.
    async fn configure_accelerometer(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        let value = match self.accel_range {
            AccelRange::G2 => 0x00,
            AccelRange::G4 => 0x08,
            AccelRange::G8 => 0x10,
            AccelRange::G16 => 0x18,
        };
        self.write_register(ACCEL_CONFIG, value)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(())
    }

    /// Configures the gyroscope range.
    async fn configure_gyroscope(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        let value = match self.gyro_range {
            GyroRange::Dps250 => 0x00,
            GyroRange::Dps500 => 0x08,
            GyroRange::Dps1000 => 0x10,
            GyroRange::Dps2000 => 0x18,
        };
        self.write_register(GYRO_CONFIG, value)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(())
    }

    /// Sets sample rate to 100Hz (8kHz / (1 + 79)).
    async fn configure_sample_rate(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        self.write_register(SMPLRT_DIV, 79)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(())
    }

    /// Writes a single register value over I2C.
    async fn write_register(&mut self, reg: u8, value: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[reg, value]).await
    }
}

// Data reading and processing
impl<I2C> Mpu6050<I2C>
where
    I2C: I2c,
{
    /// Reads raw accelerometer and gyroscope data from the sensor.
    async fn read_raw_data(&mut self) -> Result<SensorData, Mpu6050Error<I2C::Error>> {
        let mut buf = [0u8; 14];
        self.i2c
            .write_read(self.address, &[ACCEL_XOUT_H], &mut buf)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;

        let data = SensorData {
            ax: i16::from_be_bytes([buf[0], buf[1]]),
            ay: i16::from_be_bytes([buf[2], buf[3]]),
            az: i16::from_be_bytes([buf[4], buf[5]]),
            gx: i16::from_be_bytes([buf[8], buf[9]]),
            gy: i16::from_be_bytes([buf[10], buf[11]]),
            gz: i16::from_be_bytes([buf[12], buf[13]]),
        };

        // Check for extreme values that could cause overflow
        if data.ax == i16::MIN || data.ay == i16::MIN || data.az == i16::MIN ||
            data.gx == i16::MIN || data.gy == i16::MIN || data.gz == i16::MIN {
            defmt::warn!("Extreme sensor value detected: {:?}", data);
            return Err(Mpu6050Error::InvalidData);
        }

        Ok(data)
    }

    /// Reads and applies deadzone to all sensor data.
    pub async fn read_all(&mut self) -> Result<SensorData, Mpu6050Error<I2C::Error>> {
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

    /// Reads averaged sensor data over multiple samples.
    pub async fn read_avg(&mut self, samples: usize) -> Result<SensorData, Mpu6050Error<I2C::Error>> {
        if samples == 0 {
            defmt::error!("Invalid sample count: 0");
            return Err(Mpu6050Error::InvalidSamples);
        }

        let mut sum = (0i64, 0, 0, 0, 0, 0); // Use i64 to prevent overflow

        for _ in 0..samples {
            let raw = self.read_raw_data().await?;
            sum.0 += raw.ax as i64;
            sum.1 += raw.ay as i64;
            sum.2 += raw.az as i64;
            sum.3 += raw.gx as i64;
            sum.4 += raw.gy as i64;
            sum.5 += raw.gz as i64;
            embassy_time::Timer::after_micros(200).await;
        }

        Ok(SensorData {
            ax: (sum.0 / samples as i64) as i16,
            ay: (sum.1 / samples as i64) as i16,
            az: (sum.2 / samples as i64) as i16,
            gx: (sum.3 / samples as i64) as i16,
            gy: (sum.4 / samples as i64) as i16,
            gz: (sum.5 / samples as i64) as i16,
        })
    }

    /// Reads the temperature sensor in raw format (degrees Celsius = raw / 340.0 + 36.53).
    pub async fn read_temperature(&mut self) -> Result<i16, Mpu6050Error<I2C::Error>> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.address, &[TEMP_OUT_H], &mut buf)
            .await
            .map_err(|e| Mpu6050Error::I2c(e))?;
        Ok(i16::from_be_bytes([buf[0], buf[1]]))
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

    /// Applies a deadzone to filter out small jitter/noise, avoiding overflow.
    fn apply_deadzone(val: i16, threshold: i16) -> i16 {
        let val_abs = (val as i32).abs(); // Use i32 to avoid overflow on i16::MIN
        if val_abs < threshold as i32 {
            0
        } else {
            val
        }
    }
}

// Orientation detection and calibration
impl<I2C> Mpu6050<I2C>
where
    I2C: I2c,
{
    /// Recalibrates the reference pose using averaged accelerometer data.
    /// Ensures Z-axis dominates to confirm FaceUp/FaceDown orientation.
    pub async fn recalibrate_pose(&mut self) -> Result<(), Mpu6050Error<I2C::Error>> {
        let SensorData { ax, ay, az, .. } = self.read_avg(25).await?;
        let accel_scale = match self.accel_range {
            AccelRange::G2 => 16384.0,
            AccelRange::G4 => 8192.0,
            AccelRange::G8 => 4096.0,
            AccelRange::G16 => 2048.0,
        };
        let z_abs = (az as i32).abs() as f32 / accel_scale;
        let x_abs = (ax as i32).abs() as f32 / accel_scale;
        let y_abs = (ay as i32).abs() as f32 / accel_scale;

        // Check if Z-axis dominates (indicating FaceUp or FaceDown)
        if z_abs < 0.8 || z_abs < x_abs * 1.5 || z_abs < y_abs * 1.5 {
            defmt::warn!("Invalid calibration: Z-axis ({}) not dominant over X ({}) or Y ({})", z_abs, x_abs, y_abs);
            return Err(Mpu6050Error::InvalidCalibration);
        }

        self.reference = Some((ax, ay, az));
        defmt::info!("Reference pose updated: ({}, {}, {})", ax, ay, az);
        Ok(())
    }

    /// Sets a manual reference pose for testing or specific use cases.
    pub fn set_manual_reference(&mut self, ax: i16, ay: i16, az: i16) -> Result<(), Mpu6050Error<I2C::Error>> {
        let accel_scale = match self.accel_range {
            AccelRange::G2 => 16384.0,
            AccelRange::G4 => 8192.0,
            AccelRange::G8 => 4096.0,
            AccelRange::G16 => 2048.0,
        };
        let z_abs = (az as i32).abs() as f32 / accel_scale;
        let x_abs = (ax as i32).abs() as f32 / accel_scale;
        let y_abs = (ay as i32).abs() as f32 / accel_scale;

        if z_abs < 0.8 || z_abs < x_abs * 1.5 || z_abs < y_abs * 1.5 {
            defmt::warn!("Invalid manual reference: Z-axis ({}) not dominant over X ({}) or Y ({})", z_abs, x_abs, y_abs);
            return Err(Mpu6050Error::InvalidCalibration);
        }

        self.reference = Some((ax, ay, az));
        defmt::info!("Manual reference pose set: ({}, {}, {})", ax, ay, az);
        Ok(())
    }

    /// Detects the sensor's orientation using accelerometer and gyroscope data.
    pub async fn detect_orientation(&mut self) -> Result<Orientation, Mpu6050Error<I2C::Error>> {
        if !self.initialized {
            defmt::warn!("Sensor not initialized; call init() first");
            return Err(Mpu6050Error::NotInitialized);
        }

        let SensorData { ax, ay, az, gx, gy, gz, .. } = self.read_all().await?;
        let ref_vec = match self.reference {
            Some(r) => r,
            None => {
                defmt::warn!("No reference vector set");
                return Err(Mpu6050Error::NotInitialized);
            }
        };

        let current_time = embassy_time::Instant::now().as_micros();
        let delta_time = self.last_time.map(|t| (current_time - t) as f32 / 1_000_000.0);
        self.last_time = Some(current_time);

        // Normalize accelerometer data
        let accel_scale = match self.accel_range {
            AccelRange::G2 => 16384.0,
            AccelRange::G4 => 8192.0,
            AccelRange::G8 => 4096.0,
            AccelRange::G16 => 2048.0,
        };
        let normalize = |x: i16, y: i16, z: i16| -> Option<(f32, f32, f32)> {
            let xf = x as f32 / accel_scale;
            let yf = y as f32 / accel_scale;
            let zf = z as f32 / accel_scale;
            let mag_sq = xf * xf + yf * yf + zf * zf;
            if mag_sq < 0.1 || mag_sq.is_nan() || mag_sq.is_infinite() {
                defmt::warn!("Invalid magnitude in normalization: {}", mag_sq);
                return None;
            }
            let mag = mag_sq.sqrt();
            Some((xf / mag, yf / mag, zf / mag))
        };

        let ref_norm = match normalize(ref_vec.0, ref_vec.1, ref_vec.2) {
            Some(v) => v,
            None => {
                defmt::warn!("Failed to normalize reference vector");
                return Err(Mpu6050Error::InvalidData);
            }
        };
        let mut curr_norm = match normalize(ax, ay, az) {
            Some(v) => v,
            None => {
                defmt::warn!("Failed to normalize current vector");
                return Err(Mpu6050Error::InvalidData);
            }
        };

        // Integrate gyroscope data using a complementary filter
        if let Some(dt) = delta_time {
            let gyro_scale = match self.gyro_range {
                GyroRange::Dps250 => 131.0,
                GyroRange::Dps500 => 65.5,
                GyroRange::Dps1000 => 32.8,
                GyroRange::Dps2000 => 16.4,
            };
            let gx_rad = gx as f32 / gyro_scale * core::f32::consts::PI / 180.0;
            let gy_rad = gy as f32 / gyro_scale * core::f32::consts::PI / 180.0;
            let gz_rad = gz as f32 / gyro_scale * core::f32::consts::PI / 180.0;

            // Complementary filter: 99% accelerometer, 1% gyroscope for stability
            let alpha = 0.99;
            curr_norm.0 = alpha * curr_norm.0 + (1.0 - alpha) * (curr_norm.0 + gz_rad * dt);
            curr_norm.1 = alpha * curr_norm.1 + (1.0 - alpha) * (curr_norm.1 - gx_rad * dt);
            curr_norm.2 = alpha * curr_norm.2 + (1.0 - alpha) * (curr_norm.2 + gy_rad * dt);

            // Re-normalize after gyro integration
            let mag_sq = curr_norm.0 * curr_norm.0 + curr_norm.1 * curr_norm.1 + curr_norm.2 * curr_norm.2;
            if mag_sq < 0.1 || mag_sq.is_nan() || mag_sq.is_infinite() {
                defmt::warn!("Invalid magnitude after gyro integration: {}", mag_sq);
                return Err(Mpu6050Error::InvalidData);
            }
            let mag = mag_sq.sqrt();
            curr_norm = (curr_norm.0 / mag, curr_norm.1 / mag, curr_norm.2 / mag);
        }

        let dot = ref_norm.0 * curr_norm.0 + ref_norm.1 * curr_norm.1 + ref_norm.2 * curr_norm.2;
        defmt::debug!("Normalized current: ({}, {}, {}), dot: {}", curr_norm.0, curr_norm.1, curr_norm.2, dot);

        // Check Z-axis dominance first for FaceUp/FaceDown
        let z_abs = curr_norm.2.abs();
        let x_abs = curr_norm.0.abs();
        let y_abs = curr_norm.1.abs();
        if z_abs > 0.7 && z_abs > x_abs * 1.5 && z_abs > y_abs * 1.5 {
            if curr_norm.2 > 0.7 {
                return Ok(Orientation::FaceUp);
            } else if curr_norm.2 < -0.7 {
                return Ok(Orientation::FaceDown);
            }
        }

        // Fallback to X/Y checks for Portrait/Landscape
        if x_abs > y_abs && x_abs > 0.5 {
            if curr_norm.0 > 0.0 {
                Ok(Orientation::LandscapeInverted)
            } else {
                Ok(Orientation::Landscape)
            }
        } else if y_abs > 0.5 {
            if curr_norm.1 > 0.0 {
                Ok(Orientation::Portrait)
            } else {
                Ok(Orientation::PortraitInverted)
            }
        } else {
            Ok(Orientation::Unknown)
        }
    }
}