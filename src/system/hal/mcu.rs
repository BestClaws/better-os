use alloc::string::String;
use defmt::export::{display, str};
use esp_hal::gpio::Input;
use crate::system::hal::encoder::EncoderState;
use crate::system::hal::encoder::EncoderState::{Ccw, Cw};

// TODO: NOT U32 , value should be in radians/sec


pub(crate) trait Mcu {
    
    // make available all embedded hal stuff and better's own hals abstracting over
    // mcu peripherals
    
    fn init();

    
}



