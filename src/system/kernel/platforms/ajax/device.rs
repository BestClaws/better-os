use alloc::boxed::Box;
use bt_hci::controller::ExternalController;
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
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use static_cell::StaticCell;
use crate::system::hal::ambience::AsyncAmbientSensor;
use crate::system::hal::battery::AsyncBattery;
use crate::system::hal::button::{AsyncButton, ButtonDriver};
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::{AsyncEncoder};
use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::system::hal::radio::AsyncRadio;
use crate::system::vendor::boby::drivers::ambient_sensor::AmbientSensorDriver;
use crate::system::vendor::boby::drivers::battery::BatteryDriver;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;
use crate::system::vendor::espressif::drivers::radio_driver::RadioDriver;
use crate::system::vendor::invensense::drivers::mpu6050::sensor::MPU6050;

static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) static ENCODER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>> = StaticCell::new();
pub(crate) static BUTTON: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncButton>>> = StaticCell::new();
pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> = StaticCell::new();
pub(crate) static BATTERY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncBattery>>> = StaticCell::new();
pub(crate) static AMBIENT_SENSOR: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncAmbientSensor>>> = StaticCell::new();
pub(crate) static GYRO_ACCELEROMETER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>> = StaticCell::new();
pub(crate) static ADC_SHARED: StaticCell<Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>>> = StaticCell::new();

pub(crate) static RADIO: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncRadio>>> = StaticCell::new();

pub(crate) fn init_device() -> PlatformDevice<'static> {


    // initialize mcu device hal
    let peripherals = mcu::init();

    // TODO: this should be something that should be present in the kernel.
    // initialize async runtime
    // the core model of multitasking.
    let system_timer = SystemTimer::new(peripherals.SYSTIMER);
    let st_alarm = system_timer.alarm0;
    crate::system::kernel::platforms::ajax::async_runtime::init(st_alarm);



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
    let gyro_accelerometer = MPU6050::new(i2c_2);



    let mut adc_config = AdcConfig::new();
    let battery_adc_pin = adc_config.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let ambient_sensor_adc_pin = adc_config.enable_pin(peripherals.GPIO3, Attenuation::_11dB);
    let adc1 = Adc::new(peripherals.ADC1, adc_config).into_async();
    let adc: &'static mut Mutex<CriticalSectionRawMutex, Adc<ADC1, Async>> = ADC_SHARED.init(Mutex::new(adc1));



    // battery
    let battery = BatteryDriver::new(adc, battery_adc_pin);


    // ambient sensor
    let ambient_sensor = AmbientSensorDriver::new(adc, ambient_sensor_adc_pin);




    let rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let timer_group_0 = TimerGroup::new(peripherals.TIMG0);
    let radio_init = esp_wifi::init(
        timer_group_0.timer0,
        rng,
    ).unwrap();



    let radio_driver = RadioDriver::new(radio_init, peripherals.BT);





    PlatformDevice {
        encoder: Some(ENCODER.init(Mutex::new(Box::new(encoder)))),
        display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),
        button: Some(BUTTON.init(Mutex::new(Box::new(button)))),
        battery: Some(BATTERY.init(Mutex::new(Box::new(battery)))),
        ambient_sensor: Some(AMBIENT_SENSOR.init(Mutex::new(Box::new(ambient_sensor)))),
        gyro_accelerometer: Some(GYRO_ACCELEROMETER.init(Mutex::new(Box::new(gyro_accelerometer)))),
        radio: Some(RADIO.init(Mutex::new(Box::new(radio_driver))))



    }


}

