use alloc::boxed::Box;
use defmt::export::display;
use crate::system::hal::display::AsyncDisplay;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use esp_hal::i2c::master::I2c;
use esp_hal::Async;
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::prelude::*;
use ssd1306::{I2CDisplayInterface, Ssd1306Async};

// Assets

type Disp = Ssd1306Async<I2CInterface<I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>>, DisplaySize128x64, BufferedGraphicsModeAsync<DisplaySize128x64>>;

pub(crate) struct Ssd1306Driver {
    display: Disp,

}

impl Ssd1306Driver {
    pub(crate) fn init(i2c_1: I2cDevice<'static, CriticalSectionRawMutex, I2c<'static, Async>>) -> Self {
        let i2c_display = I2CDisplayInterface::new(i2c_1);
        let display = Ssd1306Async::new(i2c_display, DisplaySize128x64, DisplayRotation::Rotate180)
            .into_buffered_graphics_mode();

        Self {
            display,
        }
    }

}

#[async_trait::async_trait(?Send)]
impl AsyncDisplay for Ssd1306Driver {
    async fn init(&mut self) {
        self.display.init().await.unwrap();
    }

    async fn draw(&mut self, buffer: &[u8]) {
        let _ = self.display.draw(buffer).await;
        self.display.flush().await.unwrap();
    }

}