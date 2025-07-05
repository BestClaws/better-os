
use esp_hal::gpio::{Input};

use embedded_hal::digital::InputPin;
use embedded_hal_async::digital::Wait;
use crate::system::hal::encoder::{Encoder};

pub struct EncoderDriver {
    a_pin: Input<'static>,
    b_pin: Input<'static>,
}



impl  EncoderDriver

{
    pub(crate) fn init(a_pin: Input<'static>, b_pin: Input<'static>) -> Self {
        Self {
            a_pin,
            b_pin,
        }
    }
}

impl Encoder for EncoderDriver {

fn input_a(&mut self) -> &mut (impl InputPin + Wait) {
        &mut self.a_pin
    }

    fn input_b(&mut self) -> &mut (impl InputPin + Wait) {
        &mut self.b_pin
    }
}


