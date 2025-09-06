use alloc::boxed::Box;
use core::cmp::PartialEq;
use
embedded_hal::digital::InputPin;
// v1.0.0
use embedded_hal_async::digital::Wait;
use async_trait::async_trait;
use defmt::Format;
// v1.0.0
use embassy_time::{Duration, Instant, WithTimeout};

pub struct ButtonDriver<P: InputPin + Wait> {
    pin: P,
    last_falling_edge: Instant,
    last_state: ButtonState,
}

impl<P: InputPin + Wait> ButtonDriver<P> {
    pub fn new(pin: P) -> Self {
        Self {
            pin,
            last_falling_edge: Instant::now() + Duration::from_secs(60 * 60 * 24 * 365 * 10), // 10 years in the future
            last_state: ButtonState::Up,
        }
    }
}

#[async_trait(?Send)]
pub trait AsyncButton {
    async fn next(&mut self) -> ButtonState;
}



#[async_trait(?Send)]
impl<P: InputPin + Wait> AsyncButton for ButtonDriver<P> {
    async fn next(&mut self) -> ButtonState {

        loop {
            // repeat after 100ms
            let result = self.pin
                .wait_for_any_edge()
                .with_timeout(Duration::from_millis(100)).await;
            let current_high = self.pin.is_high().unwrap();
            let now = Instant::now();

            match result {
                Ok(Ok(_)) => {
                    if current_high {
                        self.last_state = ButtonState::Up;
                        return ButtonState::Up
                    } else {
                        self.last_falling_edge = now;
                        self.last_state = ButtonState::Down;
                        return ButtonState::Down
                    }
                },

                Ok(Err(_)) => {
                    // Error waiting for pin state change
                    defmt::error!("Error waiting for pin state change");
                },

                Err(_) => {
                    // Timeout, no state change detected
                    // Emit periodic repeat while button is held beyond threshold.
                    if self.last_state == ButtonState::Down && self.last_falling_edge < Instant::now() - Duration::from_secs(1) {
                        return ButtonState::Repeat;
                    } else {
                        continue;
                    }
                }
            }
        }
    }



}
#[derive(Debug, Format, PartialEq)]
pub(crate) enum ButtonState {
    Down,
    Up,
    /// Repeat indicates the button remains pressed beyond a threshold and generates periodic events.
    Repeat,
}
