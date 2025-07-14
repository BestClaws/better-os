#![allow(unused)]

use alloc::boxed::Box;

use async_trait::async_trait;
use crate::util::math::primitives::Quaternion;

#[async_trait(?Send)]
pub trait AsyncGyroAccelerometer {

    async fn init(&mut self);


    // async fn get_acceleration(&mut self) -> (f32, f32, f32);
    // async fn get_temperature_celsius(&mut self) -> u8;
    async fn get_orientation(&mut self) -> Quaternion;

}

