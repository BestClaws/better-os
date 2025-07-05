use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use esp_hal::{peripherals, Async};
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::prelude::*;
use static_cell::StaticCell;




pub(crate) fn foo(i2c_1: I2cDevice) {


    let i2c_disp = I2CDisplayInterface::new(i2c_1);
    
    
   
    
    
    
    
    let mut display = Ssd1306Async::new(
        i2c_disp,
        DisplaySize128x64,
        DisplayRotation::Rotate180,
    ).into_buffered_graphics_mode();
    
    
    
    display.init().await.unwrap();
    
}