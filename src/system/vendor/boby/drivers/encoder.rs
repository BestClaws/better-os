use alloc::string::String;
use defmt::export::{display, str};
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::peripherals;
use esp_hal::peripherals::{GPIO0, GPIO7, GPIO8};
use embedded_hal::digital::InputPin;
use embedded_hal_async::digital::Wait;
use crate::system::hal::encoder::Encoder;

pub struct EncoderDriver<A, B> {
    a_pin: A,
    b_pin: B,
}



impl<A, B> Encoder<A, B> for EncoderDriver<A, B>
where     A: InputPin + Wait, B: InputPin + Wait {
    fn init(a_pin: A, b_pin: B)
            -> Self {

        Self {
            a_pin,
            b_pin,
        }
    }

    fn input_a(&mut self) -> &mut A {
        &mut self.a_pin
    }

    fn input_b(&mut self) -> &mut B{
        &mut self.b_pin
    }


}