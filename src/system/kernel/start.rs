use defmt::{info,};
use embassy_executor::Spawner;
use embassy_time::Instant;
use crate::system::kernel::{platforms};

use panic_rtt_target as _;


use crate::system::services::compositor::compositor_service;
use crate::system::services::human_input;
use crate::system::services::human_input::human_input_service;
use crate::system::services::task_spawner::task_spawner_service;
// Define the static mutex for the device

pub(crate) fn start(spawner: Spawner) {
    // setup kernel level logging.
    rtt_target::rtt_init_defmt!();
    // heap support
    esp_alloc::heap_allocator!(size: 72 * 1024);
    // this is the only platform rn, so hardcode initialization
    // this should ideally give a device instead of storing in static cell
    // but the embassy tasks can't accept arguments with generics (platform device) so
    // we will use a static cell to store the device, and retreive it from there.
    let device = platforms::ajax::device::init_device();
  
    // device has already started the async runtime.
    // TODO: note: this runtime start should be done in the kernel.

    info!("[{}s] spawned  human input service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(human_input::sub::listen_encoder(device.encoder.unwrap())).unwrap();
    spawner.spawn(human_input::sub::listen_button(device.button.unwrap())).unwrap();

    info!("[{}s] spawned  compositor service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(compositor_service(device.display.unwrap())).unwrap();
    info!("[{}s] spawned  task spawner service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(task_spawner_service(spawner)).unwrap()






}





