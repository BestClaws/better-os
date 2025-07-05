
use embedded_hal::digital::InputPin;
use embedded_hal_async::digital::Wait;
use crate::system::hal::encoder::EncoderState::{Ccw, Cw};

// TODO: NOT U32 , value should be in radians/sec
pub(crate) enum EncoderState {
    Cw(u32),
    Ccw(u32)
}

pub(crate) trait Encoder {
    fn input_a(&mut self) -> &mut (impl InputPin + Wait);
    fn input_b(&mut self) -> &mut (impl InputPin + Wait);

    async fn wait_for_next_state(&mut self) -> EncoderState {
        // TODO: dont use unwrap, handle error gracefully.
        self.input_a().wait_for_falling_edge().await.unwrap();

        // TODO: refactor, debounce should be part of driver.
        embassy_time::Timer::after_millis(1).await;

        let a_high = self.input_a().is_high().unwrap();
        let b_high = self.input_b().is_high().unwrap();

        if !a_high && b_high {
            Ccw(1)
        } else if !a_high && !b_high{
            Cw(1)
        } else {
            // TODO: don't panic on hardware failure recover gracefully.
            panic!("Invalid encoder state")
        }
    }
}



