use alloc::string::String;
use defmt::export::{display, str};
use esp_hal::gpio::Input;
use crate::system::hal::encoder::EncoderState::{Ccw, Cw};

// TODO: NOT U32 , value should be in radians/sec
pub(crate) enum EncoderState {
    Cw(u32),
    Ccw(u32)
}

pub(crate) trait Encoder {

    // TODO: should be embedded-hal pins. not esp specific.
    fn init(pin_a: &mut Input, pin_b: &mut Input) -> Self;
    
    fn input_a(&mut self) -> &mut Input;
    fn input_b(&mut self) -> &mut Input;
    

    
    /// wait for the next state of encoder.
    async fn wait_for_next_state(&mut self) -> EncoderState {
        self.input_a().wait_for_falling_edge().await;
        
        // TODO: refactor, debounce should be part of driver.
        embassy_time::Timer::after_millis(1).await;
        
        display.flush().await.unwrap();

        
        if self.input_a().is_low() && self.input_b().is_high() {
            Ccw(1)
        } else if self.input_a().is_low() && self.input_b().is_low(){
            Cw(1)
        } else {
            // TODO: don't panic on hardware failure recover gracefully.
            panic!("Invalid encoder state")
        }
    }
    
}



