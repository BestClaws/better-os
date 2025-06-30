use alloc::string::String;
use defmt::export::{display, str};
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::peripherals;
use esp_hal::peripherals::{GPIO0, GPIO7, GPIO8};
use embedded_hal::digital::InputPin;
use crate::system::hal::encoder::Encoder;

pub struct EncoderDriver<'d> {
    input_a: Input<'d>,
    input_b: Input<'d>,
}

impl EncoderDriver {

}

impl Encoder for EncoderDriver {
    // todo: these should be hal pins.
    fn init(a_pin: GPIO7, b_pin: GPIO8) -> impl Encoder {
        let mut input_a = Input::new(a_pin, InputConfig::default().with_pull(Pull::Up));
        let mut input_b = Input::new(b_pin, InputConfig::default().with_pull(Pull::Up));

        Self {
            input_a,
            input_b,
        }
    }

    fn input_a(&mut self) -> &mut Input {
        &mut self.input_a
    }

    fn input_b(&mut self) -> &mut Input {
        &mut self.input_b
    }


}