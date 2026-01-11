use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::system::hal::display::PixelFormat;
use crate::system::kernel::platforms;
use crate::system::services::ambient_srv::ambient_sensor_service;
use crate::system::services::app_spawner_srv::app_spawner_service;
use crate::system::services::battery_srv::battery_service;
use crate::system::services::compositor_srv::ui_compositor_service;
use crate::system::services::input;
use crate::system::services::system_ui_srv::system_ui_gesture_task;
use crate::system::services::vibrator_srv;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::windowing::WindowManager;

use crate::system::services::gyro_accel_srv::gyro_accelerometer_service;
use crate::system::services::hps_service::hps_service;
use crate::system::services::http_service::http_service;
use crate::system::services::vibrator_srv::vibrator_service;
use panic_rtt_target as _;
use static_cell::StaticCell;

pub static COMPOSITOR: StaticCell<Mutex<CriticalSectionRawMutex, UICompositor>> = StaticCell::new();
pub static WINDOW_MANAGER: StaticCell<Mutex<CriticalSectionRawMutex, WindowManager>> =
    StaticCell::new();

pub(crate) fn start(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    // Increase heap size to account for dynamic framebuffer allocation.
    esp_alloc::heap_allocator!(size: 240 * 1024);

    let device = platforms::ajax::device::init_device();

    // Initialize the global compositor and window manager early
    let compositor_ref = COMPOSITOR.init(Mutex::new(UICompositor::new()));
    let window_manager_ref =
        WINDOW_MANAGER.init(Mutex::new(WindowManager::new(PixelFormat::Gray4)));

    // Spawn input service tasks (readers + dispatcher)
    info!(
        "[{}s] spawned input service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    // Note: current PlatformDevice has no encoder field
    if let Some(button) = device.button {
        spawner.spawn(input::button_reader_task(button)).unwrap();
    }
    if let Some(touch) = device.touch {
        spawner.spawn(input::touch_reader_task(touch)).unwrap();
    }
    spawner
        .spawn(input::input_dispatcher_task(
            compositor_ref,
            window_manager_ref,
        ))
        .unwrap();

    // Spawn compositor service
    info!(
        "[{}s] spawned compositor service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    spawner
        .spawn(ui_compositor_service(
            device.display.unwrap(),
            compositor_ref,
            window_manager_ref,
        ))
        .unwrap();

    // Dedicated System UI gesture consumer keeps acknowledgements fast.
    spawner
        .spawn(system_ui_gesture_task(compositor_ref, window_manager_ref))
        .unwrap();

    // Spawn accel service
    info!(
        "[{}s] spawned accel service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    spawner
        .spawn(gyro_accelerometer_service(
            device.gyro_accelerometer.unwrap(),
        ))
        .unwrap();

    if let Some(radio) = device.radio {
        info!(
            "[{}s] spawning HPS service",
            Instant::now().as_millis() as f32 / 1000f32
        );
        spawner.spawn(hps_service(radio)).unwrap();

        // Spawn HTTP service (sits between HTTP client and HPS)
        info!(
            "[{}s] spawning HTTP service",
            Instant::now().as_millis() as f32 / 1000f32
        );
        spawner.spawn(http_service()).unwrap();
    }

    // Spawn app spawner service
    info!(
        "[{}s] spawned app spawner service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    spawner
        .spawn(app_spawner_service(
            compositor_ref,
            window_manager_ref,
            spawner,
        ))
        .unwrap();
}
