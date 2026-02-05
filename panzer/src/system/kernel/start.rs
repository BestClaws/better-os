use defmt::{error, info, warn, Debug2Format};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::system::hal::display::PixelFormat;
use crate::system::kernel::platforms;

use crate::system::tasks::compositor_srv::window_compositor_service;
use crate::system::tasks::touch_srv::touch_service;
use panic_rtt_target as _;
use static_cell::StaticCell;

pub(crate) fn start(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    // Increase heap size to account for dynamic framebuffer allocation.
    esp_alloc::heap_allocator!(size: 190 * 1024);

    let mut device = platforms::ajax::device::init_device();

    let display = match device.display.take() {
        Some(display) => display,
        None => {
            error!("No display available; compositor service cannot start");
            return;
        }
    };

    let touch = device.touch.take();
    if touch.is_none() {
        warn!("No touch controller available; touch input will not be available");
    }

    if let Err(err) = spawner.spawn(window_compositor_service(display, touch)) {
        error!(
            "Failed to spawn window compositor service: {:?}",
            Debug2Format(&err)
        );
        return;
    }
}
