use alloc::boxed::Box;
use esp_hal::peripherals::Peripherals;
use crate::system::drivers::encoder::EncoderDriver;
use crate::system::drivers::radio::RadioDriver;
use crate::system::drivers;
use crate::system::hal::encoder::Encoder;
use crate::system::kernel::async_runtime;


pub(crate) struct Devices {
    encoder: dyn Encoder
}

pub(crate) fn start() {
    // setup kernel level logging.
    rtt_target::rtt_init_defmt!();

    // heap support
    esp_alloc::heap_allocator!(size: 72 * 1024);
    
    hw_init()
}


pub(crate) fn hw_init() {


// TODO: mcu init should convert everything to embedded-hal traits and group as Mcu type
    // initialize mcu device hal
    
    let peripherals = drivers::mcu::init();


    // initialize async runtime
    // the core model of multitasking.
    async_runtime::init(peripherals.SYSTIMER);

    let devices = init_remaining_devices(peripherals);
    
    // start system services.
    
}



pub(crate) fn init_remaining_devices(peripherals: Peripherals) -> Box<Devices> {



    


    // initialize other device hals
    let encoder  = EncoderDriver::init(peripherals.GPIO7, peripherals.GPIO8);
    
    // todo: make a hal device for this.
    let d_radio = RadioDriver::new(peripherals.RNG, peripherals.TIMG0, peripherals.RADIO_CLK);
    
    
    Box::new(Devices { encoder })
    
}
