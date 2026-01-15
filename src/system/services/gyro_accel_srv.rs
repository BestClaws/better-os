use crate::system::hal::imu::AsyncGyroAccelerometer;
use crate::util::math::primitives::{Quaternion, Vec3};
use alloc::boxed::Box;
use defmt::{debug, info};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

pub static ORIENTATION_CHANNEL: Signal<CriticalSectionRawMutex, Quaternion> = Signal::new();
static ORIENTATION_STATE: Mutex<CriticalSectionRawMutex, Option<Quaternion>> =
    Mutex::new(None);

/// Returns the latest orientation sample if available.
pub async fn latest_orientation() -> Option<Quaternion> {
    let state = ORIENTATION_STATE.lock().await;
    *state
}

/// Waits for the next orientation update from the IMU service.
pub async fn wait_for_orientation_update() -> Quaternion {
    ORIENTATION_CHANNEL.wait().await
}

#[embassy_executor::task]
pub(crate) async fn gyro_accelerometer_service(
    sensor: &'static Mutex<CriticalSectionRawMutex, Box<dyn AsyncGyroAccelerometer>>,
) {
    info!("initializing gyro accelerometer service...");
    let mut sensor_g = sensor.lock().await;
    sensor_g.init().await.unwrap();
    info!("gyro accelerometer service initialized");

    loop {
        let (x, y, z) = sensor_g.read_accel().await;
        debug!("gyro: x: {}, y: {}, z: {}", x, y, z);

        let orientation = orientation_from_accel(Vec3(x, y, z));
        {
            let mut state = ORIENTATION_STATE.lock().await;
            *state = Some(orientation);
        }
        ORIENTATION_CHANNEL.signal(orientation);

        Timer::after(Duration::from_millis(50)).await;
    }
}

fn orientation_from_accel(accel: Vec3) -> Quaternion {
    let gravity = accel.normalize();
    if gravity.length_squared() < 1.0e-6 {
        return Quaternion::identity();
    }

    let world_up = Vec3(0.0, 1.0, 0.0);
    let target = Vec3(-gravity.0, -gravity.1, -gravity.2).normalize();
    rotation_between(world_up, target)
}

fn rotation_between(from: Vec3, to: Vec3) -> Quaternion {
    let from_norm = from.normalize();
    let to_norm = to.normalize();
    let mut dot = from_norm.dot(to_norm);
    if dot.is_nan() {
        return Quaternion::identity();
    }
    dot = dot.clamp(-1.0, 1.0);

    if dot > 0.999_999 {
        return Quaternion::identity();
    }

    if dot < -0.999_999 {
        let mut axis = Vec3(1.0, 0.0, 0.0).cross(from_norm);
        if axis.length_squared() < 1.0e-6 {
            axis = Vec3(0.0, 0.0, 1.0).cross(from_norm);
        }
        axis = axis.normalize();
        return Quaternion {
            w: 0.0,
            x: axis.0,
            y: axis.1,
            z: axis.2,
        };
    }

    let cross = from_norm.cross(to_norm);
    let q = Quaternion {
        w: 1.0 + dot,
        x: cross.0,
        y: cross.1,
        z: cross.2,
    };
    q.normalize()
}
