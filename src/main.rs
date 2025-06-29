//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;


mod peripherals;
mod tasks;
mod mpu;

use bt_hci::controller::ExternalController;
use defmt::info;
use embassy_executor::Spawner;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::iso_8859_1::FONT_4X6;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive};
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::i2c;
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::{Timer, TimerGroup};
use esp_wifi::ble::controller::BleConnector;
use panic_rtt_target as _;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use trouble_host::prelude::*;



use tasks::ble::run_ble_controller;
use tasks::ticker::ticker;

use peripherals::battery::battery_task;
use peripherals::vibrator::periodic_vibration;
use peripherals::vibrator::vibrator_task;
use crate::mpu::{Mpu6050, Orientation};

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
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



    //
    // spawner.spawn(ticker()).unwrap();
    // spawner.spawn(vibrator_task(peripherals.GPIO7)).unwrap(); // Spawn the new vibration task
    // spawner.spawn(periodic_vibration()).unwrap();
    // spawner.spawn(battery_task(peripherals.ADC1, peripherals.GPIO2)).unwrap();


    use embedded_graphics::{
        mono_font::{MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        prelude::*,
        text::{Baseline, Text},
    };
    use ssd1306::prelude::*;



    let i2c = I2c::new(peripherals.I2C0.reborrow(), i2c::master::Config::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5)
        .into_async();

    // let i2c = I2CDisplayInterface::new(i2c);

    // let mut display = Ssd1306Async::new(
    //     i2c,
    //     DisplaySize128x64,
    //     DisplayRotation::Rotate0,
    // ).into_buffered_graphics_mode();



    let mut imu = Mpu6050::new(i2c);

    imu.init(None, None).await.unwrap();
    let mut last_orientation = Orientation::Unknown;

    loop {
        let orientation = imu.detect_orientation().await.unwrap();
        if orientation != last_orientation {
            info!("Orientation changed: {:?}", orientation);
            last_orientation = orientation;
        }

        embassy_time::Timer::after_millis(200).await;
    }


    //
    // display.init().await.unwrap();
    //
    // let text_style = MonoTextStyleBuilder::new()
    //     .font(&FONT_4X6)
    //     .text_color(BinaryColor::On)
    //     .build();
    //
    // Text::with_baseline("test!", Point::zero(), text_style, Baseline::Top)
    //     .draw(&mut display)
    //     .unwrap();
    //
    //
    //
    // display.flush().await.unwrap();
    //
    // let mut b1 = Input::new(peripherals.GPIO9, InputConfig::default().with_pull(Pull::Up));
    //
    // // run_ble_controller(controller).await;
    // loop {
    //     fill_bw(&mut display, BinaryColor::On).unwrap(); // white
    //     display.flush().await.unwrap();
    //     embassy_time::Timer::after(embassy_time::Duration::from_millis(1000)).await;
    //
    //     fill_bw(&mut display, BinaryColor::Off).unwrap(); // black
    //     display.flush().await.unwrap();
    //
    //     embassy_time::Timer::after(embassy_time::Duration::from_millis(1000)).await;
    // }

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

