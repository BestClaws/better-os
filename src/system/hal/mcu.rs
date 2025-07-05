
// TODO: NOT U32 , value should be in radians/sec


pub(crate) trait Mcu {
    
    // make available all embedded hal stuff and better's own hals abstracting over
    // mcu peripherals
    
    fn init();

    
}



