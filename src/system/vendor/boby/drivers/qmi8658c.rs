use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::boxed::Box;
use async_trait::async_trait;
use defmt::{debug, error, info, warn, Format};
use embassy_time::Timer;
use embedded_hal::i2c::ErrorType;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

// ============================================================================
// CONSTANTS AND REGISTER DEFINITIONS
// ============================================================================

/// QMI8658C I2C device addresses (see datasheet §12.2)
const DEVICE_ADDRESS_LOW: SevenBitAddress = 0x6A;
const DEVICE_ADDRESS_HIGH: SevenBitAddress = 0x6B;

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

/// Supported I2C address options configured via the SA0 strap.
#[derive(Clone, Copy, Debug, Format)]
pub enum Qmi8658Address {
    /// SA0 pulled low (device address 0x6A)
    Low,
    /// SA0 pulled high (device address 0x6B)
    High,
    /// Explicit 7-bit address for custom wiring
    Custom(SevenBitAddress),
}

impl Qmi8658Address {
    fn as_u8(self) -> SevenBitAddress {
        match self {
            Qmi8658Address::Low => DEVICE_ADDRESS_LOW,
            Qmi8658Address::High => DEVICE_ADDRESS_HIGH,
            Qmi8658Address::Custom(addr) => addr,
        }
    }
}

// Register addresses - following datasheet naming convention
mod registers {
    pub const WHO_AM_I: u8 = 0x00;
    pub const REVISION: u8 = 0x01;
    pub const CTRL1: u8 = 0x02;
    pub const CTRL2: u8 = 0x03;
    pub const CTRL3: u8 = 0x04;
    pub const CTRL5: u8 = 0x06;
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
    pub const CTRL1_SPI_3WIRE: u8 = 1 << 7;
    pub const CTRL1_ADDR_AUTO_INCREMENT: u8 = 1 << 6;
    pub const CTRL1_SENSOR_DISABLE: u8 = 1 << 0;

    // CTRL7 register bits - sensor enable/disable
    pub const CTRL7_SYNC_SAMPLE: u8 = 1 << 7;
    pub const CTRL7_SYS_HS: u8 = 1 << 6;
    pub const CTRL7_GYRO_SNOOZE: u8 = 1 << 4;
    pub const CTRL7_ACCEL_ENABLE: u8 = 1 << 0;
    pub const CTRL7_GYRO_ENABLE: u8 = 1 << 1;

    // CTRL5 register bits - low pass filters
    pub const CTRL5_ACCEL_LPF_ENABLE: u8 = 1 << 0;
    pub const CTRL5_ACCEL_LPF_MODE_MASK: u8 = 0b0000_0110;
    pub const CTRL5_ACCEL_LPF_MODE_SHIFT: u8 = 1;
    pub const CTRL5_GYRO_LPF_ENABLE: u8 = 1 << 4;
    pub const CTRL5_GYRO_LPF_MODE_MASK: u8 = 0b0110_0000;
    pub const CTRL5_GYRO_LPF_MODE_SHIFT: u8 = 5;

    // STATUS0 register bits - data ready flags
    pub const STATUS0_ACCEL_READY: u8 = 1 << 0;
    pub const STATUS0_GYRO_READY: u8 = 1 << 1;
}

// ============================================================================
// CONFIGURATION ENUMS
// ============================================================================

/// Accelerometer measurement range configuration (CTRL2 aFS)
#[derive(Debug, Clone, Copy, Format)]
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
#[derive(Debug, Clone, Copy, Format)]
#[repr(u8)]
pub enum AccelODR {
    /// 8000 Hz - maximum bandwidth
    Freq8000Hz = 0,
    /// 4000 Hz - reduced bandwidth
    Freq4000Hz = 1,
    /// 2000 Hz - reduced bandwidth
    Freq2000Hz = 2,
    /// 1000 Hz - high frequency sampling
    Freq1000Hz = 3,
    /// 500 Hz - balanced power and performance
    Freq500Hz = 4,
    /// 250 Hz - moderate power consumption
    Freq250Hz = 5,
    /// 125 Hz - low power mode
    Freq125Hz = 6,
    /// 62.5 Hz - low power mode
    Freq62Hz5 = 7,
    /// 31.25 Hz - low power mode
    Freq31Hz25 = 8,
    /// 128 Hz - accelerometer low power mode (gyro disabled)
    LowPower128Hz = 0xC,
    /// 21 Hz - accelerometer low power mode (gyro disabled)
    LowPower21Hz = 0xD,
    /// 11 Hz - accelerometer low power mode (gyro disabled)
    LowPower11Hz = 0xE,
    /// 3 Hz - accelerometer low power mode (gyro disabled)
    LowPower3Hz = 0xF,
}

/// Gyroscope measurement range configuration
#[derive(Debug, Clone, Copy, Format)]
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
    /// ±1024 degrees per second
    Range1024DPS = 6,
    /// ±2048 degrees per second - maximum range
    Range2048DPS = 7,
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
            GyroRange::Range2048DPS => 2048.0 / 32768.0,
        }
    }
}

/// Gyroscope output data rate configuration
#[derive(Debug, Clone, Copy, Format)]
#[repr(u8)]
pub enum GyroODR {
    /// 8000 Hz - maximum bandwidth
    Freq8000Hz = 0,
    /// 4000 Hz - reduced bandwidth
    Freq4000Hz = 1,
    /// 2000 Hz - reduced bandwidth
    Freq2000Hz = 2,
    /// 1000 Hz - high performance mode
    Freq1000Hz = 3,
    /// 500 Hz - balanced mode
    Freq500Hz = 4,
    /// 250 Hz - moderate power
    Freq250Hz = 5,
    /// 125 Hz - low power mode
    Freq125Hz = 6,
    /// 62.5 Hz - low power mode
    Freq62Hz5 = 7,
    /// 31.25 Hz - low power mode
    Freq31Hz25 = 8,
}

/// On-die low-pass filter selection (CTRL5 aLPF/gLPF)
#[derive(Debug, Clone, Copy, Format)]
#[repr(u8)]
pub enum FilterBandwidth {
    Percent2_62 = 0,
    Percent3_59 = 1,
    Percent5_32 = 2,
    Percent14_0 = 3,
}

/// Bias calibration offsets in engineering units.
#[derive(Debug, Clone, Copy, Format)]
pub struct Calibration {
    pub accel_bias: Vec3,
    pub gyro_bias: Vec3,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            accel_bias: Vec3(0.0, 0.0, 0.0),
            gyro_bias: Vec3(0.0, 0.0, 0.0),
        }
    }
}

/// Runtime configuration applied during initialization (derived from datasheet §5.4).
#[derive(Debug, Clone, Copy, Format)]
pub struct Qmi8658Config {
    pub accel_range: AccelRange,
    pub accel_odr: AccelODR,
    pub accel_lpf: Option<FilterBandwidth>,
    pub gyro_range: GyroRange,
    pub gyro_odr: GyroODR,
    pub gyro_lpf: Option<FilterBandwidth>,
    pub sync_sample: bool,
    pub high_speed_clock: bool,
    pub enable_gyro: bool,
    pub gyro_snooze: bool,
}

impl Default for Qmi8658Config {
    fn default() -> Self {
        Self {
            accel_range: AccelRange::Range4G,
            accel_odr: AccelODR::Freq500Hz,
            accel_lpf: Some(FilterBandwidth::Percent5_32),
            gyro_range: GyroRange::Range512DPS,
            gyro_odr: GyroODR::Freq500Hz,
            gyro_lpf: Some(FilterBandwidth::Percent5_32),
            sync_sample: true,
            high_speed_clock: true,
            enable_gyro: true,
            gyro_snooze: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Format)]
pub struct SensorSample {
    pub accel: Vec3,
    pub gyro: Vec3,
    pub temp_c: f32,
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

    /// Selected 7-bit address for I2C transactions
    address: SevenBitAddress,

    /// Sensor initialization state tracking
    is_initialized: bool,

    /// Current accelerometer range configuration
    accel_range: AccelRange,

    /// Current gyroscope range configuration
    gyro_range: GyroRange,

    /// Cached configuration for re-application after fault recovery
    config: Qmi8658Config,

    /// Optional calibration offsets applied to measurements
    calibration: Calibration,

    /// Accelerometer enable state
    accel_enabled: bool,

    /// Gyroscope enable state
    gyro_enabled: bool,

    /// Device revision ID for compatibility checking
    revision_id: u8,

    /// Last coherent sample fetched from the device (accel + gyro + temp)
    last_sample: Option<SensorSample>,
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
        Self::with_config(i2c_bus, Qmi8658Address::High, Qmi8658Config::default())
    }

    /// Create a driver with an explicit address selection.
    pub fn with_address(i2c_bus: I2C, address: Qmi8658Address) -> Self {
        Self::with_config(i2c_bus, address, Qmi8658Config::default())
    }

    /// Create a driver with a fully specified configuration.
    pub fn with_config(i2c_bus: I2C, address: Qmi8658Address, config: Qmi8658Config) -> Self {
        Self {
            i2c_bus,
            address: address.as_u8(),
            is_initialized: false,
            accel_range: config.accel_range,
            gyro_range: config.gyro_range,
            config,
            calibration: Calibration::default(),
            accel_enabled: false,
            gyro_enabled: false,
            revision_id: 0,
            last_sample: None,
        }
    }

    /// Update calibration offsets applied to all subsequent samples.
    pub fn set_calibration(&mut self, calibration: Calibration) {
        self.calibration = calibration;
    }

    /// Fetch the currently applied calibration.
    pub fn calibration(&self) -> Calibration {
        self.calibration
    }

    /// Retrieve the active configuration snapshot.
    pub fn config(&self) -> Qmi8658Config {
        self.config
    }

    /// Apply a new runtime configuration (requires sensor to be initialized).
    pub async fn reconfigure(&mut self, config: Qmi8658Config) -> Result<(), Qmi8658Error> {
        self.config = config;
        self.accel_range = config.accel_range;
        self.gyro_range = config.gyro_range;
        if self.is_initialized {
            self.apply_configuration().await?;
        }
        Ok(())
    }

    /// Retrieve the most recent coherent sample captured by the driver.
    pub fn last_sample(&self) -> Option<SensorSample> {
        self.last_sample
    }

    fn upsert_last_sample(&mut self, accel: Option<Vec3>, gyro: Option<Vec3>, temp: Option<f32>) {
        let mut current = self.last_sample.unwrap_or(SensorSample {
            accel: Vec3(0.0, 0.0, 0.0),
            gyro: Vec3(0.0, 0.0, 0.0),
            temp_c: 0.0,
        });

        if let Some(a) = accel {
            current.accel = a;
        }
        if let Some(g) = gyro {
            current.gyro = g;
        }
        if let Some(t) = temp {
            current.temp_c = t;
        }

        self.last_sample = Some(current);
    }

    fn orientation_from_gravity(accel: Vec3) -> Quaternion {
        let gravity = accel.normalize();
        if gravity.length_squared() < 1.0e-6 {
            return Quaternion::identity();
        }

        let world_up = Vec3(0.0, 1.0, 0.0);
        let target = Vec3(-gravity.0, -gravity.1, -gravity.2).normalize();
        Self::rotation_between(world_up, target)
    }

    fn rotation_between(from: Vec3, to: Vec3) -> Quaternion {
        let from_norm = from.normalize();
        let to_norm = to.normalize();
        let mut dot = from_norm.dot(to_norm);
        if dot.is_nan() {
            return Quaternion::identity();
        }
        dot = dot.clamp(-1.0, 1.0);

        if dot > 0.999_999 {
            return Quaternion::identity();
        }

        if dot < -0.999_999 {
            let mut axis = Vec3(1.0, 0.0, 0.0).cross(from_norm);
            if axis.length_squared() < 1.0e-6 {
                axis = Vec3(0.0, 0.0, 1.0).cross(from_norm);
            }
            axis = axis.normalize();
            return Quaternion {
                w: 0.0,
                x: axis.0,
                y: axis.1,
                z: axis.2,
            };
        }

        let cross = from_norm.cross(to_norm);
        let q = Quaternion {
            w: 1.0 + dot,
            x: cross.0,
            y: cross.1,
            z: cross.2,
        };
        q.normalize()
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
        self.write_register(registers::CTRL1, register_bits::CTRL1_ADDR_AUTO_INCREMENT)
            .await?;

        Ok(())
    }

    /// Verify device identity by reading WHO_AM_I register
    async fn verify_device_identity(&mut self) -> Result<(), Qmi8658Error> {
        let chip_id = self.read_register(registers::WHO_AM_I).await?;

        if chip_id != EXPECTED_CHIP_ID {
            error!(
                "Device ID mismatch: expected 0x{:02X}, got 0x{:02X}",
                EXPECTED_CHIP_ID, chip_id
            );
            return Err(Qmi8658Error::DeviceNotFound);
        }

        // Also read revision for future compatibility checks
        self.revision_id = self.read_register(registers::REVISION).await.unwrap_or(0);

        info!("QMI8658C detected, revision: 0x{:02X}", self.revision_id);
        Ok(())
    }

    async fn apply_configuration(&mut self) -> Result<(), Qmi8658Error> {
        debug!(
            "Applying configuration: accel_range={:?} accel_odr={:?} gyro_range={:?} gyro_odr={:?}",
            self.config.accel_range,
            self.config.accel_odr,
            self.config.gyro_range,
            self.config.gyro_odr
        );

        // Ensure auto-increment is enabled for burst transfers (datasheet Table 24 CTRL1)
        let ctrl1_value = register_bits::CTRL1_ADDR_AUTO_INCREMENT;
        self.write_register(registers::CTRL1, ctrl1_value).await?;

        // Configure accelerometer dynamic range and ODR
        let ctrl2_value = ((self.config.accel_range as u8) << 4) | (self.config.accel_odr as u8);
        self.write_register(registers::CTRL2, ctrl2_value).await?;
        self.accel_range = self.config.accel_range;

        // Configure gyroscope dynamic range and ODR
        let ctrl3_value = ((self.config.gyro_range as u8) << 4) | (self.config.gyro_odr as u8);
        self.write_register(registers::CTRL3, ctrl3_value).await?;
        self.gyro_range = self.config.gyro_range;

        // Program low-pass filter selections
        let mut ctrl5_value = 0u8;
        if let Some(mode) = self.config.accel_lpf {
            ctrl5_value |= register_bits::CTRL5_ACCEL_LPF_ENABLE;
            ctrl5_value |= (mode as u8) << register_bits::CTRL5_ACCEL_LPF_MODE_SHIFT;
        }
        if let Some(mode) = self.config.gyro_lpf {
            ctrl5_value |= register_bits::CTRL5_GYRO_LPF_ENABLE;
            ctrl5_value |= (mode as u8) << register_bits::CTRL5_GYRO_LPF_MODE_SHIFT;
        }
        self.write_register(registers::CTRL5, ctrl5_value).await?;

        // Enable requested sensors and timing behavior (CTRL7)
        let mut ctrl7_value = 0u8;
        if self.config.sync_sample {
            ctrl7_value |= register_bits::CTRL7_SYNC_SAMPLE;
        }
        if self.config.high_speed_clock {
            ctrl7_value |= register_bits::CTRL7_SYS_HS;
        }
        if self.config.gyro_snooze {
            ctrl7_value |= register_bits::CTRL7_GYRO_SNOOZE;
        }
        if self.config.enable_gyro {
            ctrl7_value |= register_bits::CTRL7_GYRO_ENABLE;
        }
        ctrl7_value |= register_bits::CTRL7_ACCEL_ENABLE;
        self.write_register(registers::CTRL7, ctrl7_value).await?;

        self.accel_enabled = true;
        self.gyro_enabled = self.config.enable_gyro;
        self.last_sample = None;

        Ok(())
    }

    /// Configure accelerometer with specified range and output data rate
    pub async fn configure_accelerometer(
        &mut self,
        range: AccelRange,
        odr: AccelODR,
    ) -> Result<(), Qmi8658Error> {
        debug!(
            "Configuring accelerometer: range={:?}, odr={:?}",
            range, odr
        );

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
        self.config.accel_range = range;
        self.config.accel_odr = odr;

        // Re-enable if it was previously enabled
        if was_enabled {
            self.enable_accelerometer().await?;
        }

        debug!("Accelerometer configuration completed");
        Ok(())
    }

    /// Configure gyroscope with specified range and output data rate
    pub async fn configure_gyroscope(
        &mut self,
        range: GyroRange,
        odr: GyroODR,
    ) -> Result<(), Qmi8658Error> {
        debug!("Configuring gyroscope: range={:?}, odr={:?}", range, odr);

        if !self.config.enable_gyro {
            warn!("Gyroscope disabled; enabling temporarily for configuration");
        }

        let was_enabled = self.gyro_enabled;
        if was_enabled {
            self.disable_gyroscope().await?;
        }

        let ctrl3_value = ((range as u8) << 4) | (odr as u8);
        self.write_register(registers::CTRL3, ctrl3_value).await?;

        self.gyro_range = range;
        self.config.gyro_range = range;
        self.config.gyro_odr = odr;

        if was_enabled {
            self.enable_gyroscope().await?;
        }

        debug!("Gyroscope configuration completed");
        Ok(())
    }

    /// Configure accelerometer and gyroscope low-pass filters (CTRL5)
    pub async fn configure_low_pass(
        &mut self,
        accel: Option<FilterBandwidth>,
        gyro: Option<FilterBandwidth>,
    ) -> Result<(), Qmi8658Error> {
        debug!("Configuring LPF: accel={:?} gyro={:?}", accel, gyro);

        let mut ctrl5 = self.read_register(registers::CTRL5).await?;

        ctrl5 &= !register_bits::CTRL5_ACCEL_LPF_MODE_MASK;
        ctrl5 &= !register_bits::CTRL5_GYRO_LPF_MODE_MASK;

        if let Some(mode) = accel {
            ctrl5 |= register_bits::CTRL5_ACCEL_LPF_ENABLE;
            ctrl5 |= (mode as u8) << register_bits::CTRL5_ACCEL_LPF_MODE_SHIFT;
            self.config.accel_lpf = Some(mode);
        } else {
            ctrl5 &= !register_bits::CTRL5_ACCEL_LPF_ENABLE;
            self.config.accel_lpf = None;
        }

        if let Some(mode) = gyro {
            ctrl5 |= register_bits::CTRL5_GYRO_LPF_ENABLE;
            ctrl5 |= (mode as u8) << register_bits::CTRL5_GYRO_LPF_MODE_SHIFT;
            self.config.gyro_lpf = Some(mode);
        } else {
            ctrl5 &= !register_bits::CTRL5_GYRO_LPF_ENABLE;
            self.config.gyro_lpf = None;
        }

        self.write_register(registers::CTRL5, ctrl5).await?;
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

    /// Enable gyroscope measurements
    pub async fn enable_gyroscope(&mut self) -> Result<(), Qmi8658Error> {
        debug!("Enabling gyroscope");

        let mut ctrl7 = self.read_register(registers::CTRL7).await?;
        ctrl7 |= register_bits::CTRL7_GYRO_ENABLE;
        self.write_register(registers::CTRL7, ctrl7).await?;

        self.gyro_enabled = true;
        self.config.enable_gyro = true;

        Timer::after_millis(10).await;

        info!("Gyroscope enabled");
        Ok(())
    }

    /// Disable gyroscope measurements
    pub async fn disable_gyroscope(&mut self) -> Result<(), Qmi8658Error> {
        debug!("Disabling gyroscope");

        let mut ctrl7 = self.read_register(registers::CTRL7).await?;
        ctrl7 &= !register_bits::CTRL7_GYRO_ENABLE;
        self.write_register(registers::CTRL7, ctrl7).await?;

        self.gyro_enabled = false;
        self.config.enable_gyro = false;

        info!("Gyroscope disabled");
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
        self.read_registers(registers::ACCEL_X_L, &mut buffer)
            .await?;

        // Convert little-endian bytes to signed 16-bit integers
        let x = i16::from_le_bytes([buffer[0], buffer[1]]);
        let y = i16::from_le_bytes([buffer[2], buffer[3]]);
        let z = i16::from_le_bytes([buffer[4], buffer[5]]);

        Ok((x, y, z))
    }

    /// Read raw gyroscope data from sensor registers
    async fn read_gyro_raw(&mut self) -> Result<(i16, i16, i16), Qmi8658Error> {
        if !self.gyro_enabled {
            return Err(Qmi8658Error::DataNotReady);
        }

        let mut buffer = [0u8; 6];
        self.read_registers(registers::GYRO_X_L, &mut buffer)
            .await?;
        let x = i16::from_le_bytes([buffer[0], buffer[1]]);
        let y = i16::from_le_bytes([buffer[2], buffer[3]]);
        let z = i16::from_le_bytes([buffer[4], buffer[5]]);

        Ok((x, y, z))
    }

    /// Read scaled gyroscope data in degrees per second
    async fn read_gyro_scaled(&mut self) -> Result<Vec3, Qmi8658Error> {
        let (raw_x, raw_y, raw_z) = self.read_gyro_raw().await?;
        let scale = self.gyro_range.scale_factor();
        let measurement = Vec3(
            raw_x as f32 * scale,
            raw_y as f32 * scale,
            raw_z as f32 * scale,
        );

        Ok(measurement.sub(self.calibration.gyro_bias))
    }

    /// Read scaled accelerometer data in g units
    async fn read_accel_scaled(&mut self) -> Result<Vec3, Qmi8658Error> {
        let (raw_x, raw_y, raw_z) = self.read_accel_raw().await?;
        let scale = self.accel_range.scale_factor();

        let measurement = Vec3(
            raw_x as f32 * scale,
            raw_y as f32 * scale,
            raw_z as f32 * scale,
        );

        Ok(measurement.sub(self.calibration.accel_bias))
    }

    /// Read temperature from internal sensor
    ///
    /// Returns temperature in degrees Celsius
    async fn read_temperature_celsius(&mut self) -> Result<f32, Qmi8658Error> {
        let mut buffer = [0u8; 2];
        self.read_registers(registers::TEMPERATURE_L, &mut buffer)
            .await?;

        let raw = i16::from_le_bytes(buffer);
        Ok(raw as f32 / 256.0)
    }

    /// Low-level register write operation with error handling
    async fn write_register(&mut self, register: u8, value: u8) -> Result<(), Qmi8658Error> {
        self.i2c_bus
            .write(self.address, &[register, value])
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)
    }

    /// Low-level single register read with error handling
    async fn read_register(&mut self, register: u8) -> Result<u8, Qmi8658Error> {
        let mut buffer = [0u8; 1];
        self.i2c_bus
            .write_read(self.address, &[register], &mut buffer)
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)?;
        Ok(buffer[0])
    }

    /// Low-level multi-register read with error handling
    async fn read_registers(
        &mut self,
        start_register: u8,
        buffer: &mut [u8],
    ) -> Result<(), Qmi8658Error> {
        self.i2c_bus
            .write_read(self.address, &[start_register], buffer)
            .await
            .map_err(|_| Qmi8658Error::BusCommunication)
    }

    /// Wait for data to become ready with timeout protection
    async fn wait_for_data_ready(
        &mut self,
        check_accel: bool,
        check_gyro: bool,
    ) -> Result<(), Qmi8658Error> {
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

        // Apply runtime configuration
        if let Err(e) = self.apply_configuration().await {
            error!("Failed to apply configuration: {:?}", e);
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
            Ok(vec) => {
                let Vec3(x, y, z) = vec;
                debug!("Accel: x={:?}g, y={:?}g, z={:?}g", x, y, z);
                self.upsert_last_sample(Some(vec), None, None);
                (x, y, z)
            }
            Err(e) => {
                warn!("Failed to read accelerometer: {:?}", e);
                if let Some(sample) = self.last_sample {
                    let Vec3(x, y, z) = sample.accel;
                    (x, y, z)
                } else {
                    (0.0, 0.0, 0.0)
                }
            }
        }
    }

    /// Read gyroscope data in degrees per second
    async fn read_gyro(&mut self) -> (f32, f32, f32) {
        if !self.config.enable_gyro {
            warn!("Gyroscope disabled in configuration");
            return (0.0, 0.0, 0.0);
        }

        if let Err(e) = self.wait_for_data_ready(false, true).await {
            warn!("Data ready timeout: {:?}", e);
            return if let Some(sample) = self.last_sample {
                let Vec3(x, y, z) = sample.gyro;
                (x, y, z)
            } else {
                (0.0, 0.0, 0.0)
            };
        }

        match self.read_gyro_scaled().await {
            Ok(vec) => {
                let Vec3(x, y, z) = vec;
                debug!("Gyro: x={:?}dps, y={:?}dps, z={:?}dps", x, y, z);
                self.upsert_last_sample(None, Some(vec), None);
                (x, y, z)
            }
            Err(e) => {
                warn!("Failed to read gyroscope: {:?}", e);
                if let Some(sample) = self.last_sample {
                    let Vec3(x, y, z) = sample.gyro;
                    (x, y, z)
                } else {
                    (0.0, 0.0, 0.0)
                }
            }
        }
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
                self.upsert_last_sample(None, None, Some(temp));
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
    async fn read_orientation(&mut self) -> Quaternion {
        if !self.is_initialized {
            warn!("Sensor not initialized");
            return Quaternion::identity();
        }

        let accel = if let Some(sample) = self.last_sample {
            sample.accel
        } else {
            if let Err(e) = self.wait_for_data_ready(true, false).await {
                warn!("Data ready timeout while fetching orientation: {:?}", e);
                return Quaternion::identity();
            }
            match self.read_accel_scaled().await {
                Ok(vec) => {
                    self.upsert_last_sample(Some(vec), None, None);
                    vec
                }
                Err(e) => {
                    warn!("Failed to read accelerometer for orientation: {:?}", e);
                    return Quaternion::identity();
                }
            }
        };

        let orientation = Self::orientation_from_gravity(accel);
        debug!(
            "Orientation estimate: w={:?} x={:?} y={:?} z={:?}",
            orientation.w, orientation.x, orientation.y, orientation.z
        );
        orientation
    }
}
