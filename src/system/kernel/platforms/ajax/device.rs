use alloc::boxed::Box;
use defmt::info;
use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;

use crate::system::vendor::boby::drivers::ssd1306::Ssd1306Driver;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex};
use embassy_sync::mutex::Mutex;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::Async;
use esp_hal::peripherals::ADC1;
use mpu6050_dmp::calibration::CalibrationParameters;
use mpu6050_dmp::sensor_async::Mpu6050;
use static_cell::StaticCell;
use crate::system::hal::ambient_sensor::AsyncAmbientSensor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::button::{AsyncButton, ButtonDriver};
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::{AsyncEncoder};
use crate::system::hal::gyro_accelerometer::AsyncGyroAccelerometer;
use crate::system::vendor::boby::drivers::ambient_sensor::AmbientSensorDriver;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;
use crate::system::vendor::boby::drivers::gyro_accelerometer::GyroAccelerometerDriver;

static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) static ENCODER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>> = StaticCell::new();
pub(crate) static BUTTON: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>> = StaticCell::new();
pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> = StaticCell::new();

pub(crate) static BATTERY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncBattery>>> = StaticCell::new();
pub(crate) static AMBIENT_SENSOR: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncAmbientSensor>>> = StaticCell::new();

pub(crate) static GYRO_ACCELEROMETER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>> = StaticCell::new();

pub(crate) static ADC_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>>> = StaticCell::new();


pub(crate) fn init_device() -> PlatformDevice<'static> {


    // initialize mcu device hal
    let peripherals = mcu::init();

    // TODO: this should be something that should be present in the kernel.
    // initialize async runtime
    // the core model of multitasking.
    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    let time_base = timer0.alarm0;
    crate::system::kernel::platforms::ajax::async_runtime::init(time_base);



    // INIT OTHER DEVICES

    // INIT ENCODER
    let encoder_a_pin = Input::new(peripherals.GPIO7, InputConfig::default().with_pull(Pull::Up));
    let encoder_b_pin = Input::new(peripherals.GPIO8, InputConfig::default().with_pull(Pull::Up));
    let button_pin = Input::new(peripherals.GPIO9, InputConfig::default().with_pull(Pull::Up));

    let encoder  = EncoderDriver::new(encoder_a_pin, encoder_b_pin);

    let button  = ButtonDriver::new(button_pin);


    // INIT I2C BUS
    let i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5)
        .into_async();

    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);
    let i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> = I2cDevice::new(i2c);

    let i2c_2: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> = I2cDevice::new(i2c);


    // INIT DISPLAY
    let display = Ssd1306Driver::init(i2c_1);

    // INIT GYRO ACCELEROMETER
    let gyro_accelerometer = GyroAccelerometerDriver::new(i2c_2);


    // todo: make a hal device for this.
    // let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);

    let mut adc_config = AdcConfig::new();
    let mut battery_adc_pin = adc_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let mut ambient_sensor_adc_pin = adc_config.enable_pin(peripherals.GPIO3, Attenuation::_11dB);
    let adc1 = Adc::new(peripherals.ADC1, adc_config).into_async();
    let adc: &'static mut Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>> = ADC_SHARED.init(Mutex::new(adc1));



    // battery
    let battery = BatteryDriver::new(adc, battery_adc_pin);


    // ambient sensor
    let ambient_sensor = AmbientSensorDriver::new(adc, ambient_sensor_adc_pin);










    PlatformDevice {
        encoder: Some(ENCODER.init(Mutex::new(Box::new(encoder)))),
        display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),
        button: Some(BUTTON.init(Mutex::new(Box::new(button)))),
        battery: Some(BATTERY.init(Mutex::new(Box::new(battery)))),
        ambient_sensor: Some(AMBIENT_SENSOR.init(Mutex::new(Box::new(ambient_sensor)))),
        gyro_accelerometer: Some(GYRO_ACCELEROMETER.init(Mutex::new(Box::new(gyro_accelerometer)))),


    }


}

