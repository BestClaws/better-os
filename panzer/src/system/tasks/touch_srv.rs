//! Touch Controller Service
//!
//! Background task that polls the CST816S touch controller and injects
//! touch events into the input system.

use crate::system::app_shell::AppShell;
use crate::system::input::InputEvent;
use crate::system::vendor::chipone::cst816s::Cst816s;
use defmt::{debug, info};
use embassy_time::{Duration, Timer};
use embedded_hal_async::i2c::I2c;

/// Touch service polling interval (milliseconds)
const TOUCH_POLL_INTERVAL_MS: u64 = 10; // 100Hz polling rate

/// Touch service task
///
/// Continuously polls the touch controller and injects touch events
/// into the AppShell input queue.
pub async fn touch_service<I2C, E>(mut touch: Cst816s<I2C>, app_shell: &mut AppShell)
where
    I2C: I2c<Error = E>,
    E: defmt::Format,
{
    info!("Starting touch service");

    // Initialize touch controller
    if let Err(e) = touch.init().await {
        defmt::error!("Failed to initialize touch controller: {:?}", e);
        return;
    }

    info!("Touch controller initialized, starting polling loop");

    loop {
        // Poll touch controller
        match touch.read_touch().await {
            Ok(Some(point)) => {
                // Queue touch event
                app_shell.queue_input(InputEvent::Touch {
                    x: point.x,
                    y: point.y,
                    pressed: point.pressed,
                });

                // Log gesture if detected
                if point.gesture != crate::system::vendor::chipone::cst816s::Gesture::None {
                    debug!("Touch gesture: {:?}", point.gesture);
                }
            }
            Ok(None) => {
                // No touch detected
            }
            Err(e) => {
                defmt::error!("Touch read error: {:?}", e);
                // Continue polling despite errors
            }
        }

        // Wait before next poll
        Timer::after_millis(TOUCH_POLL_INTERVAL_MS).await;
    }
}
