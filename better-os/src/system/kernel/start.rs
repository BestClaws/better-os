use defmt::{error, info, warn, Debug2Format};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::system::hal::display::PixelFormat;
use crate::system::input::types::KeyCode;
use crate::system::kernel::platforms;
use crate::system::services::tasks::ambient_srv::ambient_sensor_service;
use crate::system::services::tasks::app_spawner_srv::app_spawner_service;
use crate::system::services::tasks::audio_srv::{audio_service, audio_service_unavailable};
use crate::system::services::tasks::battery_srv::battery_service;
use crate::system::services::tasks::compositor_srv::ui_compositor_service;
use crate::system::services::input;
use crate::system::services::tasks::system_ui_srv::system_ui_gesture_task;
use crate::system::services::tasks::vibrator_srv;
use crate::system::ui::compositor::UICompositor;
use crate::system::ui::windowing::WindowManager;

use crate::system::services::tasks::gyro_accel_srv::gyro_accelerometer_service;
use crate::system::services::hps_service::hps_service;
use crate::system::services::tasks::http_service::http_service;
use crate::system::services::tasks::rtc_srv::rtc_service;
use crate::system::services::tasks::rtc_sync_srv::rtc_sync_service;
use crate::system::services::tasks::vibrator_srv::vibrator_service;
use panic_rtt_target as _;
use static_cell::StaticCell;

pub static COMPOSITOR: StaticCell<Mutex<CriticalSectionRawMutex, UICompositor>> = StaticCell::new();
pub static WINDOW_MANAGER: StaticCell<Mutex<CriticalSectionRawMutex, WindowManager>> =
    StaticCell::new();

pub(crate) fn start(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    // Increase heap size to account for dynamic framebuffer allocation.
    esp_alloc::heap_allocator!(size: 207 * 1024);

    let mut device = platforms::ajax::device::init_device();

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
    if let Some(button) = device.button.take() {
        if let Err(err) = spawner.spawn(input::button_reader_task(button, KeyCode::Ok)) {
            warn!("Failed to spawn button reader: {:?}", Debug2Format(&err));
        }
    }
    if let Some(app_switch) = device.app_switch_button.take() {
        if let Err(err) = spawner.spawn(input::button_reader_task(app_switch, KeyCode::NextApp)) {
            warn!(
                "Failed to spawn app-switch reader: {:?}",
                Debug2Format(&err)
            );
        }
    }
    if let Some(touch) = device.touch.take() {
        if let Err(err) = spawner.spawn(input::touch_reader_task(touch)) {
            warn!("Failed to spawn touch reader: {:?}", Debug2Format(&err));
        }
    }
    if let Err(err) = spawner.spawn(input::input_dispatcher_task(
        compositor_ref,
        window_manager_ref,
    )) {
        error!("Failed to spawn input dispatcher: {:?}", Debug2Format(&err));
        return;
    }

    // Spawn compositor service
    info!(
        "[{}s] spawned compositor service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    let display = match device.display.take() {
        Some(display) => display,
        None => {
            error!("No display available; compositor service cannot start");
            return;
        }
    };

    if let Err(err) = spawner.spawn(ui_compositor_service(
        display,
        compositor_ref,
        window_manager_ref,
    )) {
        error!(
            "Failed to spawn compositor service: {:?}",
            Debug2Format(&err)
        );
        return;
    }

    // Dedicated System UI gesture consumer keeps acknowledgements fast.
    if let Err(err) = spawner.spawn(system_ui_gesture_task(compositor_ref, window_manager_ref)) {
        warn!(
            "Failed to spawn system UI gesture task: {:?}",
            Debug2Format(&err)
        );
    }

    // Spawn accel service
    info!(
        "[{}s] spawned accel service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    match device.gyro_accelerometer.take() {
        Some(imu) => {
            if let Err(err) = spawner.spawn(gyro_accelerometer_service(imu)) {
                error!("Failed to spawn gyro service: {:?}", Debug2Format(&err));
            }
        }
        None => warn!("Gyro/accelerometer not present; orientation features disabled"),
    }

    if let Some(audio) = device.audio.take() {
        if let Err(err) = spawner.spawn(audio_service(audio)) {
            warn!("Failed to spawn audio service: {:?}", Debug2Format(&err));
        }
    } else {
        warn!("Audio sink not present; audio app disabled (stub active)");
        if let Err(err) = spawner.spawn(audio_service_unavailable()) {
            warn!("Failed to spawn audio stub: {:?}", Debug2Format(&err));
        }
    }

    if let Some(radio) = device.radio.take() {
        info!(
            "[{}s] spawning HPS service",
            Instant::now().as_millis() as f32 / 1000f32
        );
        if let Err(err) = spawner.spawn(hps_service(radio)) {
            warn!("Failed to spawn HPS service: {:?}", Debug2Format(&err));
        }

        // Spawn HTTP service (sits between HTTP client and HPS)
        info!(
            "[{}s] spawning HTTP service",
            Instant::now().as_millis() as f32 / 1000f32
        );
        if let Err(err) = spawner.spawn(http_service()) {
            warn!("Failed to spawn HTTP service: {:?}", Debug2Format(&err));
        }

        if device.rtc.is_some() {
            info!(
                "[{}s] spawning RTC sync service",
                Instant::now().as_millis() as f32 / 1000f32
            );
            if let Err(err) = spawner.spawn(rtc_sync_service()) {
                warn!("Failed to spawn RTC sync service: {:?}", Debug2Format(&err));
            }
        }
    }

    let rtc = device.rtc.take();

    if let Some(rtc) = rtc {
        info!(
            "[{}s] spawning RTC service",
            Instant::now().as_millis() as f32 / 1000f32
        );
        if let Err(err) = spawner.spawn(rtc_service(rtc)) {
            warn!("Failed to spawn RTC service: {:?}", Debug2Format(&err));
        }
    }

    // Spawn app spawner service
    info!(
        "[{}s] spawned app spawner service",
        Instant::now().as_millis() as f32 / 1000f32
    );
    if let Err(err) = spawner.spawn(app_spawner_service(
        compositor_ref,
        window_manager_ref,
        spawner,
    )) {
        error!(
            "Failed to spawn app spawner service: {:?}",
            Debug2Format(&err)
        );
    }
}
