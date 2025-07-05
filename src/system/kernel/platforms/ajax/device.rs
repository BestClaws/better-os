use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_hal::Async;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use static_cell::StaticCell;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) fn  get_device() -> PlatformDevice<EncoderDriver> {


    // initialize mcu device hal
    let peripherals = mcu::init();

    // TODO: this should be something that should be present in the kernel.
    // initialize async runtime
    // the core model of multitasking.

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    let time_base = timer0.alarm0;
    crate::system::kernel::platforms::ajax::async_runtime::init(time_base);



    // initialize other device hals
    let input_a = Input::new(peripherals.GPIO7, InputConfig::default().with_pull(Pull::Up));
    let input_b = Input::new(peripherals.GPIO8, InputConfig::default().with_pull(Pull::Up));

    let encoder  = EncoderDriver::init(input_a, input_b);



    let i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5)
        .into_async();

    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);

    let i2c_1 = I2cDevice::new(i2c);

   

    // todo: make a hal device for this.
    // let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);


    PlatformDevice {
        encoder: Some(encoder)
    }
}

