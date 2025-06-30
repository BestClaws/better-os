//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;


mod peripherals;
mod tasks;
mod mpu;

use alloc::format;
use alloc::string::String;
use core::cell::RefCell;
use crate::i2c::master::Config;
use bt_hci::controller::ExternalController;
use defmt::export::str;
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_graphics::mono_font::iso_8859_1::{FONT_4X6, FONT_6X9};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::ascii::FONT_5X7;
use embedded_graphics::mono_font::iso_8859_16::FONT_8X13_BOLD;
use embedded_hal_async::digital::Wait;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::{i2c, Async};
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::{Timer, TimerGroup};
use esp_wifi::ble::controller::BleConnector;
use mpu6050_dmp::calibration::CalibrationParameters;
use mpu6050_dmp::quaternion::Quaternion;
use mpu6050_dmp::sensor_async::Mpu6050;
use mpu6050_dmp::yaw_pitch_roll::YawPitchRoll;
use panic_rtt_target as _;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use static_cell::StaticCell;
use trouble_host::prelude::*;


use tasks::ticker::ticker;

use peripherals::battery::battery_task;
use peripherals::vibrator::periodic_vibration;



#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {



    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let mut peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("[main] Embassy initialized");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);

    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timer1.timer0,
        rng.clone(),
        peripherals.RADIO_CLK,
    ).unwrap();

    let connector = BleConnector::new(&init, peripherals.BT);
    let controller: ExternalController<_, 20> = ExternalController::new(connector);



    let mut ra = Input::new(peripherals.GPIO7, InputConfig::default().with_pull(Pull::Up));
    let mut rb = Input::new(peripherals.GPIO8, InputConfig::default().with_pull(Pull::Up));









    use embedded_graphics::{
        mono_font::MonoTextStyleBuilder,
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Baseline, Text},
    };
    use ssd1306::prelude::*;




    static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2c<Async>>> = StaticCell::new();


    let i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5)
        .into_async();


    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);


    let i2c_disp = I2CDisplayInterface::new(I2cDevice::new(i2c));

    let mut display = Ssd1306Async::new(
        i2c_disp,
        DisplaySize128x64,
        DisplayRotation::Rotate180,
    ).into_buffered_graphics_mode();



    display.init().await.unwrap();




    let mut sensor = Mpu6050::new(I2cDevice::new(i2c), mpu6050_dmp::address::Address::default()).await.unwrap();


    let temp = sensor.temperature().await.unwrap();
    info!("Temperature: {}°C", temp.celsius());

    let mut delay  = embassy_time::Delay;
    sensor.initialize_dmp(&mut delay).await.unwrap();
    info!("DMP Firmware Initialized");

    // Read raw accelerometer data (uncalibrated)
    // The accelerometer measures linear acceleration in three axes (X, Y, Z)
    // Values will be imprecise until calibration is performed
    let accel_data = sensor.accel().await.unwrap();
    info!(
        "Accelerometer [mg]: x={}, y={}, z={}",
        accel_data.x() as i32,
        accel_data.y() as i32,
        accel_data.z() as i32
    );

    // Read raw gyroscope data (uncalibrated)
    // The gyroscope measures angular velocity in three axes (X, Y, Z)
    // Values will have drift and bias until calibration is performed
    let gyro_data = sensor.gyro().await.unwrap();
    info!(
        "Gyroscope [deg/s]: x={}, y={}, z={}",
        gyro_data.x() as i32,
        gyro_data.y() as i32,
        gyro_data.z() as i32
    );

    // Configure sensor calibration parameters
    // AccelFullScale options: G2, G4, G8, G16 (higher means larger range, lower precision)
    // GyroFullScale options: Deg250, Deg500, Deg1000, Deg2000 (degrees/second range)
    // ReferenceGravity: XN, XP, YN, YP, ZN, ZP (axis and direction of gravity during calibration)
    let calibration_params = CalibrationParameters::new(
        mpu6050_dmp::accel::AccelFullScale::G2,
        mpu6050_dmp::gyro::GyroFullScale::Deg2000,
        mpu6050_dmp::calibration::ReferenceGravity::ZN,
    );

    // info!("Calibrating Sensor");
    // sensor
    //     .calibrate(&mut delay, &calibration_params)
    //     .await
    //     .unwrap();
    // info!("Sensor Calibrated");

    // Read the accelerometer data from the mpu6050-dmp sensor again after calibration
    let accel_data = sensor.accel().await.unwrap();
    info!(
        "Accelerometer [mg]: x={}, y={}, z={}",
        accel_data.x() as i32,
        accel_data.y() as i32,
        accel_data.z() as i32
    );

    // Read the gyroscope data from the mpu6050-dmp sensor again after calibration
    let gyro_data = sensor.gyro().await.unwrap();
    info!(
        "Gyroscope [deg/s]: x={}, y={}, z={}",
        gyro_data.x() as i32,
        gyro_data.y() as i32,
        gyro_data.z() as i32
    );

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_8X13_BOLD)
        .text_color(BinaryColor::On)
        .build();


    let mut str = String::from("");


    loop {

        ra.wait_for_falling_edge().await;
        embassy_time::Timer::after_millis(1).await;



        display.flush().await.unwrap();



        if ra.is_low() && rb.is_high() {
            str = String::from("cw");
        } else if ra.is_low() && rb.is_low(){

            str = String::from("ccw");
        }

        display.clear_buffer();
        Text::with_baseline(str.as_str(), Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        embassy_time::Timer::after_millis(5).await;


    }



    // Main loop: Read sensor data every second
    // - Accelerometer: returns g-force per axis, including gravity
    // - Gyroscope: returns rotational velocity in degrees/second
    // - Temperature: returns degrees Celsius
    loop {
        let (accel, gyro, temp) = (
            sensor.accel().await.unwrap(),
            sensor.gyro().await.unwrap(),
            sensor.temperature().await.unwrap().celsius(),
        );
        info!("Sensor Readings:");
        info!(
            "  Accelerometer [mg]: x={}, y={}, z={}",
            accel.x() as i32,
            accel.y() as i32,
            accel.z() as i32
        );
        info!(
            "  Gyroscope [deg/s]: x={}, y={}, z={}",
            gyro.x() as i32,
            gyro.y() as i32,
            gyro.z() as i32
        );


        let str = format!(
                "  Gyro: x={}, y={}, z={}",
                gyro.x() as i32,
                gyro.y() as i32,
                gyro.z() as i32
        );
        info!("  Temperature: {}°C", temp);
        display.clear_buffer();
        Text::with_baseline(str.as_str(), Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        display.flush().await.unwrap();




        

        embassy_time::Timer::after_millis(1000).await;
    }












}


fn fill_bw<D>(display: &mut D, color: BinaryColor) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let style = PrimitiveStyle::with_fill(color);

    Rectangle::new(Point::zero(), display.bounding_box().size)
        .into_styled(style)
        .draw(display)?;

    Ok(())
}


