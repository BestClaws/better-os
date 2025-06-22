//! BLE HID Keyboard example on ESP32 (three buttons → F7, F8, F9)
#![no_std]
#![no_main]

extern crate alloc;


mod peripherals;
mod tasks;


use bt_hci::controller::ExternalController;
use defmt::{info};
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use panic_rtt_target as _;
use trouble_host::{prelude::*};


use tasks::ticker::ticker;
use tasks::ble::run_ble_controller;

use peripherals::vibrator::periodic_vibration;
use peripherals::battery::battery_task;
use peripherals::vibrator::vibrator_task;



#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
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


    spawner.spawn(ticker()).unwrap();
    spawner.spawn(vibrator_task(peripherals.GPIO7)).unwrap(); // Spawn the new vibration task
    spawner.spawn(periodic_vibration()).unwrap();
    spawner.spawn(battery_task(peripherals.ADC1, peripherals.GPIO2)).unwrap();


    run_ble_controller(controller).await;

}


