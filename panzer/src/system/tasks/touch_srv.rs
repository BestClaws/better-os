//! Touch Controller Service (Interrupt-Driven)
//!
//! High-priority task that responds to FT3x68 touch interrupt events
//! and immediately injects touch events into the input system.
//!
//! This runs independently of the frame loop for responsive touch input.

use crate::system::input::InputEvent;
use crate::system::vendor::focaltech::ft3x68::Ft3x68;
use defmt::{debug, error, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::Timer;
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::I2c;

/// Touch event channel capacity
const TOUCH_QUEUE_SIZE: usize = 16;

/// Shared touch event channel (to be initialized once)
pub static TOUCH_CHANNEL: Channel<CriticalSectionRawMutex, InputEvent, TOUCH_QUEUE_SIZE> =
    Channel::new();

/// Touch service task (interrupt-driven)
///
/// Waits for touch interrupt, reads controller, and sends events to channel.
/// This runs at high priority independent of frame rate.
#[embassy_executor::task]
pub async fn touch_service(
    mut touch: Ft3x68<esp_hal::i2c::master::I2c<'static, esp_hal::Async>>,
    mut int_pin: esp_hal::gpio::Input<'static>,
) {
    info!("Starting interrupt-driven touch service");

    // Initialize touch controller
    if let Err(e) = touch.init().await {
        error!("Failed to initialize touch controller: {:?}", e);
        return;
    }

    info!("Touch controller initialized, waiting for interrupts on GPIO15");

    let sender = TOUCH_CHANNEL.sender();

    loop {
        // Wait for touch interrupt (falling edge - active low)
        // This waits for the pin to transition from high to low, avoiding
        // immediately triggering if the pin is already low
        int_pin.wait_for_falling_edge().await;

        // Read touch points
        match touch.read_touch().await {
            Ok(Some(point)) => {
                // Convert TouchEvent to pressed boolean
                let pressed =
                    point.event != crate::system::vendor::focaltech::ft3x68::TouchEvent::LiftUp;

                // Send to channel (non-blocking)
                let event = InputEvent::Touch {
                    x: point.x,
                    y: point.y,
                    pressed,
                };

                // Send without blocking (drop if queue full)
                if sender.try_send(event).is_err() {
                    debug!("Touch event queue full, dropping event");
                }
            }
            Ok(None) => {
                // No touch detected
            }
            Err(e) => {
                error!("Touch read error: {:?}", e);
                Timer::after_millis(10).await;
            }
        }

        // Wait for interrupt line to go back high (touch released)
        // This prevents rapid re-triggering while finger is still on screen
        int_pin.wait_for_high().await;
        
        // Small debounce delay
        Timer::after_millis(5).await;
    }
}

/// Get the touch event channel sender for use in other tasks
pub fn get_touch_sender() -> Sender<'static, CriticalSectionRawMutex, InputEvent, TOUCH_QUEUE_SIZE>
{
    TOUCH_CHANNEL.sender()
}
