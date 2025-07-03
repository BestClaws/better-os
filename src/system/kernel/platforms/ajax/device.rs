use crate::system::hal::encoder::Encoder;
use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;
use alloc::boxed::Box;
use bt_hci::param::LeConnRole::Peripheral;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::peripherals::Peripherals;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;

pub(crate) fn get_device() -> Box<PlatformDevice> {


    // initialize mcu device hal
    let peripherals = mcu::init();

    // TODO: this should be something that should be present in the kernel.
    // initialize async runtime
    // the core model of multitasking.
    crate::system::kernel::platforms::ajax::async_runtime::init(peripherals.SYSTIMER);



    // initialize other device hals
    let mut input_a = Input::new(peripherals.GPIO7, InputConfig::default().with_pull(Pull::Up));
    let mut input_b = Input::new(peripherals.GPIO8, InputConfig::default().with_pull(Pull::Up));

    let encoder  = EncoderDriver::init(input_a, input_b);

    // todo: make a hal device for this.
    // let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);


    Box::new(PlatformDevice { encoder })
}

