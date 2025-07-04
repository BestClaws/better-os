use crate::system::kernel::platform::PlatformDevice;
use crate::system::vendor::espressif::mcu;
use alloc::boxed::Box;
use bt_hci::param::LeConnRole::Peripheral;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::timer::systimer::SystemTimer;
use crate::system::vendor::boby::drivers::encoder::EncoderDriver;

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

    // todo: make a hal device for this.
    // let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);


    PlatformDevice { encoder }
}

