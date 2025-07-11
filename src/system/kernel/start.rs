use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::system::kernel::platforms;
use crate::system::services::ambient_sensor::ambient_sensor_service;
use crate::system::services::battery::battery_service;
use crate::system::services::compositor_service::compositor_service;
use crate::system::services::app_spawner_service::app_spawner_service;
use crate::system::services::human_input;
use crate::system::ui::compositor::UICompositor;

use panic_rtt_target as _;
use static_cell::StaticCell;
use crate::system::ui::input_channels::init_channels;

pub static COMPOSITOR: StaticCell<Mutex<CriticalSectionRawMutex, UICompositor>> = StaticCell::new();

pub(crate) fn start(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let device = platforms::ajax::device::init_device();

    init_channels();

    // Initialize the global compositor
    let compositor_ref = COMPOSITOR.init(Mutex::new(UICompositor::new()));

    // Spawn input services
    info!("[{}s] spawned human input service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(human_input::sub::listen_encoder(device.encoder.unwrap())).unwrap();
    spawner.spawn(human_input::sub::listen_button(device.button.unwrap())).unwrap();

    // Spawn sensors
    info!("[{}s] spawned ambient sensor service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(ambient_sensor_service(device.ambient_sensor.unwrap())).unwrap();

    info!("[{}s] spawned battery service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(battery_service(device.battery.unwrap())).unwrap();

    // Spawn compositor service
    info!("[{}s] spawned compositor service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(compositor_service(device.display.unwrap(), compositor_ref)).unwrap();

    // Spawn app spawner service
    info!("[{}s] spawned app spawner service", Instant::now().as_millis() as f32 / 1000f32);
    spawner.spawn(app_spawner_service(compositor_ref,spawner)).unwrap();
}
