use alloc::boxed::Box;
use esp_hal::peripherals::Peripherals;

use crate::system::hal::encoder::Encoder;
use crate::system::kernel::platforms::ajax::device::get_device;

pub(crate) fn start() {
    // setup kernel level logging.
    rtt_target::rtt_init_defmt!();

    // heap support
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let device = get_device();


}


pub(crate) fn hw_init() {


}




