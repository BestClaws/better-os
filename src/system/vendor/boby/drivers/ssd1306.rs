use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use crate::system::hal::display::Display;

pub(crate) struct Ssd1306Driver {
    display: Ssd1306Async<I2CInterface<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>,
}
impl  Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {


        let i2c_disp = I2CDisplayInterface::new(i2c_1);


        let display = Ssd1306Async::new(
            i2c_disp,
            DisplaySize128x64,
            DisplayRotation::Rotate180,
        ).into_buffered_graphics_mode();


        Self {
            display,
        }

        // display.init().await.unwrap();

    }
}


impl Display for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
    }
}
