use alloc::boxed::Box;
use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;

use crate::system::vendor::boby::drivers::ssd1306::Ssd1306Driver;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex};
use embassy_sync::mutex::Mutex;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::Async;
use static_cell::StaticCell;
use crate::system::hal::display::AsyncDisplay;
use crate::system::hal::encoder::{AsyncEncoder, EncoderDriver};

static I2C_BUS: StaticCell<Mutex<CriticalSectionRawMutex, I2c<Async>>> = StaticCell::new();

pub(crate) static ENCODER: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncEncoder>>> = StaticCell::new();
pub(crate) static DISPLAY: StaticCell<Mutex<CriticalSectionRawMutex, Box<dyn AsyncDisplay>>> = StaticCell::new();


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
    let input_a = Input::new(peripherals.GPIO7, InputConfig::default().with_pull(Pull::Up));
    let input_b = Input::new(peripherals.GPIO8, InputConfig::default().with_pull(Pull::Up));

    let encoder  = EncoderDriver::new(input_a, input_b);


    // INIT I2C BUS
    let i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default().with_frequency(Rate::from_khz(400)))
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5)
        .into_async();

    let i2c = Mutex::new(i2c);
    let i2c = I2C_BUS.init(i2c);
    let i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>> = I2cDevice::new(i2c);


    // INIT DISPLAY
    let display = Ssd1306Driver::init(i2c_1);

    // todo: make a hal device for this.
    // let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);


    PlatformDevice {
        encoder: Some(ENCODER.init(Mutex::new(Box::new(encoder)))),
        display: Some(DISPLAY.init(Mutex::new(Box::new(display)))),

    }


}

