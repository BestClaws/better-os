
use crate::system::kernel::platforms::ajax::device::get_device;

use panic_rtt_target as _;
pub(crate) fn start() {
    // setup kernel level logging.
    rtt_target::rtt_init_defmt!();

    // heap support
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let device = get_device();


}


pub(crate) fn hw_init() {


}




