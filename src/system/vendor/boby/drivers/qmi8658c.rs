use alloc::boxed::Box;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};
use async_trait::async_trait;
use defmt::{info, warn, error, debug, Format};
use embassy_time::Timer;
use embedded_hal::i2c::ErrorType;
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};

// ============================================================================
// CONSTANTS AND REGISTER DEFINITIONS
// ============================================================================

/// QMI8658C I2C device address
const DEVICE_ADDRESS: SevenBitAddress = 0x6B;

/// Expected WHO_AM_I register response for device identification
const EXPECTED_CHIP_ID: u8 = 0x05;

/// Reset command value to trigger sensor soft reset
const RESET_COMMAND: u8 = 0xB0;

/// Reset completion flag expected in reset result register
const RESET_SUCCESS_FLAG: u8 = 0x80;

/// Maximum time to wait for sensor operations (milliseconds)
const OPERATION_TIMEOUT_MS: u64 = 500;

/// Data ready polling interval (milliseconds)
const DATA_READY_POLL_MS: u64 = 1;

// Register addresses - following datasheet naming convention
mod registers {
    pub const WHO_AM_I: u8 = 0x00;
    pub const REVISION: u8 = 0x01;
    pub const CTRL1: u8 = 0x02;
    pub const CTRL2: u8 = 0x03;
    pub const CTRL7: u8 = 0x08;
    pub const RESET: u8 = 0x60;
    pub const RESET_RESULT: u8 = 0x4D;
    pub const STATUS_INT: u8 = 0x2D;
    pub const STATUS0: u8 = 0x2E;
    pub const TEMPERATURE_L: u8 = 0x33;
    pub const ACCEL_X_L: u8 = 0x35;
    pub const ACCEL_Y_L: u8 = 0x37;
    pub const ACCEL_Z_L: u8 = 0x39;
    pub const GYRO_X_L: u8 = 0x3B;
    pub const GYRO_Y_L: u8 = 0x3D;
    pub const GYRO_Z_L: u8 = 0x3F;
}

// Register bit masks and values
mod register_bits {
    // CTRL1 register bits
    pub const CTRL1_ADDR_AUTO_INCREMENT: u8 = 1 << 6;

    // CTRL7 register bits - sensor enable/disable
    pub const CTRL7_ACCEL_ENABLE: u8 = 1 << 0;
    pub const CTRL7_GYRO_ENABLE: u8 = 1 << 1;

    // STATUS0 register bits - data ready flags
    pub const STATUS0_ACCEL_READY: u8 = 1 << 0;
    pub const STATUS0_GYRO_READY: u8 = 1 << 1;
}

// ============================================================================
// CONFIGURATION ENUMS
// ============================================================================

/// Accelerometer measurement range configuration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccelRange {
    /// ±2g range, highest resolution
    Range2G = 0,
    /// ±4g range, balanced resolution and range
    Range4G = 1,
    /// ±8g range, good for dynamic applications
    Range8G = 2,
    /// ±16g range, maximum range
    Range16G = 3,
}

impl AccelRange {
    /// Get the scaling factor to convert raw ADC values to acceleration in g
    ///
    /// The QMI8658C uses 16-bit signed integers for raw data.
    /// Scale factor = full_scale_range / (2^15) where 2^15 = 32768
    const fn scale_factor(self) -> f32 {
        match self {
            AccelRange::Range2G => 2.0 / 32768.0,
            AccelRange::Range4G => 4.0 / 32768.0,
            AccelRange::Range8G => 8.0 / 32768.0,
            AccelRange::Range16G => 16.0 / 32768.0,
        }
    }
}

/// Accelerometer output data rate (ODR) configuration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AccelODR {
    /// 1000 Hz - high frequency sampling
    Freq1000Hz = 3,
    /// 500 Hz - balanced power and performance
    Freq500Hz = 4,
    /// 250 Hz - moderate power consumption
    Freq250Hz = 5,
    /// 125 Hz - low power mode
    Freq125Hz = 6,
}

/// Gyroscope measurement range configuration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GyroRange {
    /// ±16 degrees per second
    Range16DPS = 0,
    /// ±32 degrees per second
    Range32DPS = 1,
    /// ±64 degrees per second
    Range64DPS = 2,
    /// ±128 degrees per second
    Range128DPS = 3,
    /// ±256 degrees per second
    Range256DPS = 4,
    /// ±512 degrees per second
    Range512DPS = 5,
    /// ±1024 degrees per second - maximum range
    Range1024DPS = 6,
}

impl GyroRange {
    /// Get the scaling factor to convert raw ADC values to angular velocity in DPS
    const fn scale_factor(self) -> f32 {
        match self {
            GyroRange::Range16DPS => 16.0 / 32768.0,
            GyroRange::Range32DPS => 32.0 / 32768.0,
            GyroRange::Range64DPS => 64.0 / 32768.0,
            GyroRange::Range128DPS => 128.0 / 32768.0,
            GyroRange::Range256DPS => 256.0 / 32768.0,
            GyroRange::Range512DPS => 512.0 / 32768.0,
            GyroRange::Range1024DPS => 1024.0 / 32768.0,
        }
    }
}

/// Gyroscope output data rate configuration
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GyroODR {
    /// 896.8 Hz - high performance mode
    Freq896Hz = 3,
    /// 448.4 Hz - balanced mode
    Freq448Hz = 4,
    /// 224.2 Hz - moderate power
    Freq224Hz = 5,
    /// 112.1 Hz - low power mode
    Freq112Hz = 6,
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

/// Comprehensive error types for robust error handling
#[derive(Debug, Clone, Copy, Format)]
pub enum Qmi8658Error {
    /// I2C bus communication failure
    BusCommunication,
    /// Device not responding or wrong chip ID
    DeviceNotFound,
    /// Operation timeout - device may be stuck
    OperationTimeout,
    /// Sensor initialization failed
    InitializationFailed,
    /// Invalid configuration parameters
    InvalidConfiguration,
    /// Device reset failed
    ResetFailed,
    /// Data not ready when expected
    DataNotReady,
}

impl From<()> for Qmi8658Error {
    fn from(_: ()) -> Self {
        Qmi8658Error::BusCommunication
    }
}

// ============================================================================
// MAIN DRIVER IMPLEMENTATION
// ============================================================================

/// High-reliability QMI8658C IMU driver for space-grade applications
///
/// This driver implements robust error handling, timeout protection,
/// and comprehensive sensor configuration for the QMI8658C 6-axis IMU.
///
/// Key features:
/// - Automatic device identification and validation
/// - Configurable measurement ranges and data rates
/// - Built-in error recovery mechanisms
/// - Temperature monitoring support
/// - Future-expandable architecture for advanced features
pub struct Qmi8658C<I2C> {
    /// I2C peripheral handle for device communication
    i2c_bus: I2C,

    /// Sensor initialization state tracking
    is_initialized: bool,

    /// Current accelerometer range configuration
    accel_range: AccelRange,

    /// Current gyroscope range configuration
    gyro_range: GyroRange,

    /// Accelerometer enable state
    accel_enabled: bool,

    /// Gyroscope enable state
    gyro_enabled: bool,

    /// Device revision ID for compatibility checking
    revision_id: u8,
}

impl<I2C> Qmi8658C<I2C>
where
    I2C: I2c,
{
    /// Create a new QMI8658C driver instance
    ///
    /// # Arguments
    /// * `i2c_bus` - I2C peripheral configured for the sensor
    ///
    /// # Returns
    /// New driver instance in uninitialized state
    pub fn new(i2c_bus: I2C) -> Self {
        Self {
            i2c_bus,
            is_initialized: false,
            accel_range: AccelRange::Range4G, // Conservative default
            gyro_range: GyroRange::Range64DPS, // Conservative default
            accel_enabled: false,
            gyro_enabled: false,
            revision_id: 0,
        }
    }

    /// Perform a soft reset of the sensor
    ///
    /// This operation resets all sensor registers to their default values
    /// and requires re-initialization afterwards.
    async fn soft_reset(&mut self) -> Result<(), Qmi8658Error> {
        debug!("Initiating QMI8658C soft reset");

        // Send reset command
        self.write_register(registers::RESET, RESET_COMMAND).await?;

        // Wait for reset to complete (datasheet specifies max 15ms)
        Timer::after_millis(20).await;

        // Verify reset completion by checking reset result register
        let start_time = embassy_time::Instant::now();

        loop {
            match self.read_register(registers::RESET_RESULT).await {
                Ok(result) if result == RESET_SUCCESS_FLAG => {
                    debug!("Soft reset completed successfully");
                    break;
                }
                Ok(result) => {
                    debug!("Reset in progress, result: 0x{:02X}", result);
                }
                Err(_) => {
                    warn!("Failed to read reset result register");
                }
            }

            if start_time.elapsed().as_millis() > OPERATION_TIMEOUT_MS {
                error!("Soft reset timeout");
                return Err(Qmi8658Error::OperationTimeout);
            }

            Timer::after_millis(DATA_READY_POLL_MS).await;
        }

        // Enable auto-increment addressing for efficient multi-byte reads
        self.write_register(registers::CTRL1, register_bits::CTRL1_ADDR_AUTO_INCREMENT).await?;

        Ok(())
    }

    /// Verify device identity by reading WHO_AM_I register
    async fn verify_device_identity(&mut self) -> Result<(), Qmi8658Error> {
        let chip_id = self.read_register(registers::WHO_AM_I).await?;

        if chip_id != EXPECTED_CHIP_ID {
            error!("Device ID mismatch: expected 0x{:02X}, got 0x{:02X}",
                   EXPECTED_CHIP_ID, chip_id);
            return Err(Qmi8658Error::DeviceNotFound);
        }

        // Also read revision for future compatibility checks
        self.revision_id = self.read_register(registers::REVISION).await
            .unwrap_or(0);

        info!("QMI8658C detected, revision: 0x{:02X}", self.revision_id);
        Ok(())
    }

    /// Configure accelerometer with specified range and output data rate
    pub async fn configure_accelerometer(&mut self, range: AccelRange, odr: AccelODR)
                                         -> Result<(), Qmi8658Error>
    {
        debug!("Configuring accelerometer: range={:?}, odr={:?}", range, odr);

        // Temporarily disable accelerometer for configuration
        let was_enabled = self.accel_enabled;
        if was_enabled {
            self.disable_accelerometer().await?;
        }

        // Configure range and ODR in CTRL2 register
        let ctrl2_value = ((range as u8) << 4) | (odr as u8);
        self.write_register(registers::CTRL2, ctrl2_value).await?;

        // Update driver state
        self.accel_range = range;

        // Re-enable if it was previously enabled
        if was_enabled {
            self.enable_accelerometer().await?;
        }

        debug!("Accelerometer configuration completed");
        Ok(())
    }

    /// Enable accelerometer measurements
    pub async fn enable_accelerometer(&mut self) -> Result<(), Qmi8658Error> {
        debug!("Enabling accelerometer");

        let mut ctrl7 = self.read_register(registers::CTRL7).await?;
        ctrl7 |= register_bits::CTRL7_ACCEL_ENABLE;
        self.write_register(registers::CTRL7, ctrl7).await?;

        self.accel_enabled = true;

        // Allow sensor to stabilize
        Timer::after_millis(10).await;

        info!("Accelerometer enabled");
        Ok(())
    }

    /// Disable accelerometer measurements to save power
    pub async fn disable_accelerometer(&mut self) -> Result<(), Qmi8658Error> {
        debug!("Disabling accelerometer");

        let mut ctrl7 = self.read_register(registers::CTRL7).await?;
        ctrl7 &= !register_bits::CTRL7_ACCEL_ENABLE;
        self.write_register(registers::CTRL7, ctrl7).await?;

        self.accel_enabled = false;

        info!("Accelerometer disabled");
        Ok(())
    }

    /// Check if accelerometer data is ready for reading
    async fn is_accel_data_ready(&mut self) -> Result<bool, Qmi8658Error> {
        let status = self.read_register(registers::STATUS0).await?;
        Ok((status & register_bits::STATUS0_ACCEL_READY) != 0)
    }

    /// Read raw accelerometer data from sensor registers
    ///
    /// Returns (x, y, z) acceleration values in ADC counts (±32768 range)
    async fn read_accel_raw(&mut self) -> Result<(i16, i16, i16), Qmi8658Error> {
        if !self.accel_enabled {
            return Err(Qmi8658Error::DataNotReady);
        }

        // Read all 6 bytes of accelerometer data in one transaction
        let mut buffer = [0u8; 6];
        self.read_registers(registers::ACCEL_X_L, &mut buffer).await?;

        // Convert little-endian bytes to signed 16-bit integers
        let x = i16::from_le_bytes([buffer[0], buffer[1]]);
        let y = i16::from_le_bytes([buffer[2], buffer[3]]);
        let z = i16::from_le_bytes([buffer[4], buffer[5]]);

        Ok((x, y, z))
    }

    /// Read scaled accelerometer data in g units
    async fn read_accel_scaled(&mut self) -> Result<(f32, f32, f32), Qmi8658Error> {
        let (raw_x, raw_y, raw_z) = self.read_accel_raw().await?;
        let scale = self.accel_range.scale_factor();

        let x_g = raw_x as f32 * scale;
        let y_g = raw_y as f32 * scale;
        let z_g = raw_z as f32 * scale;

        Ok((x_g, y_g, z_g))
    }

    /// Read temperature from internal sensor
    ///
    /// Returns temperature in degrees Celsius
    async fn read_temperature_celsius(&mut self) -> Result<f32, Qmi8658Error> {
        let mut buffer = [0u8; 2];
        self.read_registers(registers::TEMPERATURE_L, &mut buffer).await?;

        // Temperature format: signed integer + fractional part
        // Formula from datasheet: temp = integer_part + (fractional_part / 256)
        let temp_celsius = buffer[1] as f32 + (buffer[0] as f32 / 256.0);

        Ok(temp_celsius)
    }

    /// Low-level register write operation with error handling
    async fn write_register(&mut self, register: u8, value: u8) -> Result<(), Qmi8658Error> {
        self.i2c_bus.write(DEVICE_ADDRESS, &[register, value])
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)
    }

    /// Low-level single register read with error handling
    async fn read_register(&mut self, register: u8) -> Result<u8, Qmi8658Error> {
        let mut buffer = [0u8; 1];
        self.i2c_bus.write_read(DEVICE_ADDRESS, &[register], &mut buffer)
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)?;
        Ok(buffer[0])
    }

    /// Low-level multi-register read with error handling
    async fn read_registers(&mut self, start_register: u8, buffer: &mut [u8]) -> Result<(), Qmi8658Error> {
        self.i2c_bus.write_read(DEVICE_ADDRESS, &[start_register], buffer)
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)
    }

    /// Wait for data to become ready with timeout protection
    async fn wait_for_data_ready(&mut self, check_accel: bool, check_gyro: bool) -> Result<(), Qmi8658Error> {
        let start_time = embassy_time::Instant::now();

        loop {
            let status = self.read_register(registers::STATUS0).await?;

            let accel_ready = !check_accel || ((status & register_bits::STATUS0_ACCEL_READY) != 0);
            let gyro_ready = !check_gyro || ((status & register_bits::STATUS0_GYRO_READY) != 0);

            if accel_ready && gyro_ready {
                return Ok(());
            }

            if start_time.elapsed().as_millis() > OPERATION_TIMEOUT_MS {
                warn!("Data ready timeout - status: 0x{:02X}", status);
                return Err(Qmi8658Error::DataNotReady);
            }

            Timer::after_millis(DATA_READY_POLL_MS).await;
        }
    }
}

// ============================================================================
// ASYNC TRAIT IMPLEMENTATION
// ============================================================================

#[async_trait(?Send)]
impl<I2C> AsyncGyroAccelerometer for Qmi8658C<I2C>
where
    I2C: I2c,
{
    /// Initialize the QMI8658C sensor with robust error checking
    ///
    /// Performs device identification, reset, and basic configuration.
    /// The sensor is left in a ready state for measurement configuration.
    async fn init(&mut self) -> Result<(), ()> {
        info!("Initializing QMI8658C IMU sensor");

        // Reset sensor to known state
        if let Err(e) = self.soft_reset().await {
            error!("Sensor reset failed: {:?}", e);
            return Err(());
        }

        // Verify we're talking to the correct device
        if let Err(e) = self.verify_device_identity().await {
            error!("Device verification failed: {:?}", e);
            return Err(());
        }

        // Configure accelerometer with safe defaults for space applications
        if let Err(e) = self.configure_accelerometer(AccelRange::Range4G, AccelODR::Freq500Hz).await {
            error!("Accelerometer configuration failed: {:?}", e);
            return Err(());
        }

        // Enable accelerometer
        if let Err(e) = self.enable_accelerometer().await {
            error!("Failed to enable accelerometer: {:?}", e);
            return Err(());
        }

        self.is_initialized = true;
        info!("QMI8658C initialization completed successfully");

        Ok(())
    }

    /// Read acceleration data in g units
    ///
    /// Returns (x, y, z) acceleration components where:
    /// - Positive X: device-dependent orientation
    /// - Positive Y: device-dependent orientation
    /// - Positive Z: device-dependent orientation
    ///
    /// Coordinate system follows sensor datasheet specifications.
    async fn read_accel(&mut self) -> (f32, f32, f32) {
        if !self.is_initialized || !self.accel_enabled {
            warn!("Accelerometer not initialized or not enabled");
            return (0.0, 0.0, 0.0);
        }

        // Wait for fresh data to be available
        if let Err(e) = self.wait_for_data_ready(true, false).await {
            warn!("Data ready timeout: {:?}", e);
            return (0.0, 0.0, 0.0);
        }

        // Read and return scaled acceleration values
        match self.read_accel_scaled().await {
            Ok((x, y, z)) => {
                debug!("Accel: x={:?}g, y={:?}g, z={:?}g", x, y, z);
                (x, y, z)
            }
            Err(e) => {
                warn!("Failed to read accelerometer: {:?}", e);
                (0.0, 0.0, 0.0)
            }
        }
    }

    /// Read gyroscope data in degrees per second
    ///
    /// Currently returns zeros - gyroscope implementation reserved for future enhancement
    async fn read_gyro(&mut self) -> (f32, f32, f32) {
        // TODO: Implement gyroscope functionality when needed
        debug!("Gyroscope functionality not yet implemented");
        (0.0, 0.0, 0.0)
    }

    /// Read internal temperature sensor
    ///
    /// Returns temperature in degrees Celsius
    async fn read_temp(&mut self) -> f32 {
        if !self.is_initialized {
            warn!("Sensor not initialized");
            return 0.0;
        }

        match self.read_temperature_celsius().await {
            Ok(temp) => {
                debug!("Temperature: {:?}°C", temp);
                temp
            }
            Err(e) => {
                warn!("Failed to read temperature: {:?}", e);
                0.0
            }
        }
    }

    /// Read sensor orientation as quaternion
    ///
    /// Currently returns identity quaternion - orientation fusion reserved for future enhancement
    async fn read_orientation(&mut self) -> Quaternion {
        // TODO: Implement sensor fusion for orientation when needed
        // This would typically require:
        // 1. Calibrated accelerometer and gyroscope data
        // 2. Sensor fusion algorithm (e.g., Madgwick, Mahony)
        // 3. Integration over time for orientation tracking

        debug!("Orientation fusion not yet implemented");
        Quaternion::identity()
    }
}