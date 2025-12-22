#![allow(unused)]

use crate::util::math::primitives::Quaternion;
use alloc::boxed::Box;
use async_trait::async_trait;

/// Async interface for motion sensors with accel+gyro (and orientation if supported).
#[async_trait(?Send)]
pub trait AsyncGyroAccelerometer {
    /// Initialize sensor, set up registers, check WHO_AM_I.
    async fn init(&mut self) -> Result<(), ()>;

    /// Read acceleration in g.
    async fn read_accel(&mut self) -> (f32, f32, f32);

    /// Read angular velocity in dps.
    async fn read_gyro(&mut self) -> (f32, f32, f32);

    /// Read temperature in °C.
    async fn read_temp(&mut self) -> f32;

    /// Read orientation as quaternion (if available, otherwise identity).
    async fn read_orientation(&mut self) -> Quaternion;
}
